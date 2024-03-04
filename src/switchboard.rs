use std::{collections::{HashMap, HashSet}, net::UdpSocket, sync::{mpsc::{self, Receiver, Sender, TryRecvError}, Arc, Mutex}, thread};
use common::comm::{BoardId, ChannelType, DataMessage, DataPoint, Measurement, NodeMapping, Sequence, Unit, VehicleState};
use jeflog::warn;
use crate::state::SharedState;

enum BoardCommunications {
  Init(BoardId, UdpSocket),
  HeartbeatAck(BoardId),
  Sam(BoardId, Vec<DataPoint>),
  Bsm(BoardId)
}

enum HeartBeat {
  Send(BoardId),
  Create(BoardId),
  Acknowledged(BoardId)
}

// TODO replace all unwrap() with proper error handling
// TODO error handle all UDP sends
// TODO error handle all Option

/// one-shot thread spawner, begins switchboard logic
pub fn run(bind_address: &str, state: &SharedState) -> Sender<(BoardId, Sequence)> {
  let (tx, rx) = mpsc::channel::<(BoardId, Sequence)>();
  thread::spawn(start_switchboard(bind_address, state, rx));
  tx
}

/// constantly checks main binding for board data, handles board initalization and data encoding
fn listen(bind_address: &str, board_tx: Sender<Option<BoardCommunications>>) -> impl FnOnce() -> () {
  let binding = bind_address.to_owned();

  move || {
    let bind_address = binding.as_str();
    let mut buffer = vec![0; 1024];
    let home_socket = UdpSocket::bind(bind_address).unwrap();
    let mut established_sockets = HashSet::new();

    loop {
      let (size, incoming_address) = home_socket.recv_from(&mut buffer).unwrap();

      if size > buffer.len() {
        buffer.resize(size, 0);
        continue;
      }

      let raw_data = postcard::from_bytes::<DataMessage>(&mut buffer[..size]).unwrap();

      board_tx.send(match raw_data {
        DataMessage::Establish(board_id, address) => {
          if established_sockets.contains(&incoming_address) {
            continue;
          }
          established_sockets.insert(incoming_address);

          let write_socket = UdpSocket::bind(address).unwrap();

          let value = DataMessage::FlightEstablishAck(None);
          postcard::to_slice(&value, &mut buffer).unwrap();
          write_socket.send(&buffer);

          Some(BoardCommunications::Init(board_id, write_socket))
        },
        DataMessage::Sam(board_id, datapoints) => Some(BoardCommunications::Sam(board_id, datapoints.to_vec())),
        DataMessage::Bms(board_id) => Some(BoardCommunications::Bsm(board_id)),
        DataMessage::HeartbeatAck(board_id) => Some(BoardCommunications::HeartbeatAck(board_id)),
        _ => None
      }).unwrap();
    }
  }
}

/// owns sockets and SharedState, changes must be sent via mpsc channel
fn start_switchboard(bind_address: &str, state: &SharedState, sequence_rx: Receiver<(BoardId, Sequence)>) -> impl FnOnce() -> () {
  let mappings = state.mappings.clone();
  let vehicle_state = state.vehicle_state.clone();
  let mut sockets: HashMap<BoardId, UdpSocket> = HashMap::new();
  let (board_tx, board_rx) = mpsc::channel::<Option<BoardCommunications>>();
  let (heartbeat_tx, heartbeat_rx) = mpsc::channel::<HeartBeat>();

  thread::spawn(listen(bind_address, board_tx));
  thread::spawn(monitor(heartbeat_rx));

  move || {
    loop {
      match board_rx.try_recv() {
        Ok(Some(BoardCommunications::Init(board_id, socket))) => { 
          sockets.insert(board_id.to_string(), socket); 
          heartbeat_tx.send(HeartBeat::Create(board_id)).unwrap();
        },
        Ok(Some(BoardCommunications::Sam(board_id, datapoints)))  => {
          process_sam_data(vehicle_state.clone(), mappings.clone(), board_id.clone(), datapoints);
          heartbeat_tx.send(HeartBeat::Acknowledged(board_id)).unwrap();
        },
        Ok(Some(BoardCommunications::Bsm(board_id))) => {
          warn!("Recieved BSM data from board {board_id}."); 
          heartbeat_tx.send(HeartBeat::Acknowledged(board_id)).unwrap();
        },
        Ok(Some(BoardCommunications::HeartbeatAck(board_id))) => { heartbeat_tx.send(HeartBeat::Acknowledged(board_id)); },
        Ok(None) => { warn!("Unknown data recieved from board!"); },
        Err(TryRecvError::Disconnected) => { /* TODO figure out what to do when disconnected */ },
        Err(TryRecvError::Empty) => {}
      };

      match sequence_rx.try_recv() {
        Ok((board_id, sequence)) => {
          let mut buf: Vec<u8> = vec![0; std::mem::size_of::<Sequence>() + 10];

          sockets.get(&board_id).unwrap().send(postcard::to_slice(&sequence, &mut buf).unwrap()); 
        },
        Err(TryRecvError::Disconnected) => { /* TODO figure out what to do when disconnected */ },
        Err(TryRecvError::Empty) => {}
      };
    }
  }
}

fn monitor(heartbeat_rx: Receiver<HeartBeat>) -> impl FnOnce() -> () {
  let clocks: Arc<Mutex<HashMap<BoardId, u32>>> = Arc::new(Mutex::new(HashMap::new()));

  thread::spawn(beat(clocks.clone()));
  
  move || {

  }
}

fn beat(clocks: Arc<Mutex<HashMap<BoardId, u32>>>) -> impl FnOnce() -> () {
  move || {
    let mut clocks = clocks.lock().unwrap();

    for clock in clocks.keys() {
      clocks.entry(clock.to_string()).and_modify(|value| *value += 1);
    }
  }
}

fn process_sam_data(vehicle_state: Arc<Mutex<VehicleState>>, mappings: Arc<Mutex<Vec<NodeMapping>>>, board_id: BoardId, data_points: Vec<DataPoint>) {
	let mut vehicle_state = vehicle_state.lock().unwrap();

	let mappings = mappings.lock().unwrap();

	for data_point in data_points {
		let mapping = mappings.iter().find(|mapping|
			data_point.channel == mapping.channel
			&& data_point.channel_type == mapping.channel_type
			&& board_id == mapping.board_id
		);

		if let Some(mapping) = mapping {
			let mut unit = mapping.channel_type.unit();
			let mut value = data_point.value;

			// apply linear transformations to current loop and differential signal channels
			// if the max and min are supplied by the mappings. otherwise, default back to volts.
			if mapping.channel_type == ChannelType::CurrentLoop {
				if let (Some(max), Some(min)) = (mapping.max, mapping.min) {
					// formula for converting voltage into psi for our PTs
					value = (value - 0.8) / 3.2 * (max - min) + min - mapping.calibrated_offset;
				} else {
					unit = Unit::Volts;
				}
			} else if mapping.channel_type == ChannelType::DifferentialSignal {
				if let (Some(_max), Some(_min)) = (mapping.max, mapping.min) {
					// TODO: implement formula for converting voltage into value for differential signal devices (typically load cells)
				} else {
					unit = Unit::Volts;
				}
			}

			vehicle_state.sensor_readings.insert(mapping.text_id.clone(), Measurement { value, unit });
		}
	}
	// TODO: push channel bursts into log file.
}	