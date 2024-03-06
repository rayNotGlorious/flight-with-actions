use std::{collections::{HashMap, HashSet}, net::UdpSocket, sync::{mpsc::{self, Receiver, Sender, TryRecvError}, Arc, Mutex}, thread, time::Duration};
use common::comm::{BoardId, ChannelType, DataMessage, DataPoint, Measurement, NodeMapping, Sequence, Unit, VehicleState};
use jeflog::{fail, warn};
use crate::state::SharedState;

/// Milliseconds of inactivity before we sent a heartbeat
const HEARTBEAT_TIMEOUT_MS: u32 = 200;
/// How many heartbeats should be sent before we consider the data board to be disconnected
const HEARTBEAT_MAX_TIMEOUT: u8 = 3;

enum BoardCommunications {
  Init(BoardId, UdpSocket),
  HeartbeatAck(BoardId),
  Sam(BoardId, Vec<DataPoint>),
  Bsm(BoardId)
}

// TODO replace all unwrap() with proper error handling
// TODO error handle all UDP sends
// TODO error handle all Option

/// one-shot thread spawner, begins switchboard logic
pub fn run(home_socket: UdpSocket, state: &SharedState) -> Sender<(BoardId, Sequence)> {
  let (tx, rx) = mpsc::channel::<(BoardId, Sequence)>();
  thread::spawn(start_switchboard(home_socket, state, rx));
  tx
}

/// constantly checks main binding for board data, handles board initalization and data encoding
fn listen(home_socket: UdpSocket, board_tx: Sender<Option<BoardCommunications>>) -> impl FnOnce() -> () {
  move || {
    let mut buffer = vec![0; 1024];
    
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
fn start_switchboard(home_socket: UdpSocket, state: &SharedState, sequence_rx: Receiver<(BoardId, Sequence)>) -> impl FnOnce() -> () {
  let mappings = state.mappings.clone();
  let vehicle_state = state.vehicle_state.clone();
  let mut sockets: HashMap<BoardId, UdpSocket> = HashMap::new();
  let mut timers: HashMap<BoardId, (u32, u8)> = HashMap::new();
  let (board_tx, board_rx) = mpsc::channel::<Option<BoardCommunications>>();

  thread::spawn(listen(home_socket, board_tx));

  move || {
    loop {
      match board_rx.try_recv() {
        Ok(Some(BoardCommunications::Init(board_id, socket))) => { 
          sockets.insert(board_id.to_string(), socket);

          timers.insert(board_id, (0, 0));
        },
        Ok(Some(BoardCommunications::Sam(board_id, datapoints)))  => {
          process_sam_data(vehicle_state.clone(), mappings.clone(), board_id.clone(), datapoints);
          
          if let Some(timer) = timers.get_mut(&board_id) {
            reset(&board_id, timer);
          } else {
            warn!("Cannot find timer for board with id of {board_id}!");
          }
        },
        Ok(Some(BoardCommunications::Bsm(board_id))) => {
          warn!("Recieved BSM data from board {board_id}."); 

          if let Some(timer) = timers.get_mut(&board_id) {
            reset(&board_id, timer);
          } else {
            warn!("Cannot find timer for board with id of {board_id}!");
          }
        },
        Ok(Some(BoardCommunications::HeartbeatAck(board_id))) => {
          if let Some(timer) = timers.get_mut(&board_id) {
            reset(&board_id, timer);
          } else {
            warn!("Cannot find timer for board with id of {board_id}!");
          }
        },
        Ok(None) => { warn!("Unknown data recieved from board!"); },
        Err(TryRecvError::Disconnected) => { warn!("Lost connection to listen() channel. This isn't supposed to happen."); },
        Err(TryRecvError::Empty) => {}
      };

      match sequence_rx.try_recv() {
        Ok((board_id, sequence)) => 'a: {
          let mut buf: Vec<u8> = vec![0; 1024];

          if let Err(e) = postcard::to_slice(&sequence, &mut buf) {
            warn!("postcard returned this error when attempting to serialize sequence {:#?}: {e}", sequence);
            break 'a;
          }
          
          if let Some(socket) = sockets.get(&board_id) {
            if let Err(e) = socket.send(&buf) {
              warn!("Couldn't send sequence to socket {:#?}: {e}", socket);
            }
          } else {
            warn!("Couldn't find socket with board ID {board_id} in sockets HashMap.");
          }
        },
        Err(TryRecvError::Disconnected) => { warn!("Lost connection to listen() channel. This isn't supposed to happen."); },
        Err(TryRecvError::Empty) => {}
      };
      
      for (board_id, timer) in timers.iter_mut() {
        timer.0 += 1;
        if timer.0 > HEARTBEAT_TIMEOUT_MS {
          timer.0 = 0;
          timer.1 += 1;

          if timer.1 > HEARTBEAT_MAX_TIMEOUT {
            disconnection_handler(&board_id);
          } else {
            let mut buf: Vec<u8> = vec![0; 1024];

            
            if let Err(e) = postcard::to_slice(&DataMessage::FlightHeartbeat, &mut buf) {
              warn!("postcard returned this error when attempting to serialize DataMessage::FlightHeartbeat: {e}");
              continue;
            }

            if let Some(socket) = sockets.get(board_id) {
              if let Err(e) = socket.send(&buf) {
                warn!("Couldn't send sequence to socket {:#?}: {e}", socket);
              }
            } else {
              warn!("Couldn't find socket with board ID {board_id} in sockets HashMap.");
            }
          }
        }
      }

      thread::sleep(Duration::from_millis(1));
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

			vehicle_state.sensor_readings.insert(mapping.text_id, Measurement { value, unit });
		}
	}
	// TODO: push channel bursts into log file.
}

fn disconnection_handler(board_id: &str) {
  fail!("{board_id} isn't responding!.");
}

fn reset(board_id: &str, timer: &mut (u32, u8)) {
  if timer.1 > 3 {
    warn!("{board_id} has reconnected!");
  }

  timer.0 = 0;
  timer.1 = 0;
}