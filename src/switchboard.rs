use std::{collections::{HashMap, HashSet}, io, net::{SocketAddr, UdpSocket}, sync::{mpsc::{self, Receiver, Sender, TryRecvError}, Arc, Mutex}, thread, time::Duration};
use common::comm::{BoardId, SamControlMessage, ChannelType, DataMessage, DataPoint, Measurement, NodeMapping, Sequence, Unit, VehicleState};
use jeflog::{task, fail, warn, pass};

/// Milliseconds of inactivity before we sent a heartbeat
const BOARD_TIMEOUT_MS: u32 = 50;
const HEARTBEAT_INTERVAL_MS: u32 = 50;

enum BoardCommunications {
  Init(BoardId, SocketAddr),
  Sam(BoardId, Vec<DataPoint>),
  Bsm(BoardId)
}

/// One-shot thread spawner, begins switchboard logic.
pub fn run(home_socket: UdpSocket, mappings: Arc<Mutex<Vec<NodeMapping>>>, vehicle_state: Arc<Mutex<VehicleState>>) -> Result<Sender<(BoardId, SamControlMessage)>, io::Error> {
  let (tx, rx) = mpsc::channel::<(BoardId, SamControlMessage)>();
  thread::spawn(start_switchboard(home_socket, mappings, vehicle_state, rx)?);
  Ok(tx)
}

/// owns sockets and SharedState, changes must be sent via mpsc channel
fn start_switchboard(home_socket: UdpSocket, mappings: Arc<Mutex<Vec<NodeMapping>>>, vehicle_state: Arc<Mutex<VehicleState>>, control_rx: Receiver<(BoardId, SamControlMessage)>) -> Result<impl FnOnce() -> (), io::Error> {
  let mappings = mappings.clone();
  let vehicle_state = vehicle_state.clone();
  let mut sockets: HashMap<BoardId, SocketAddr> = HashMap::new();
  let mut timers: HashMap<BoardId, u32> = HashMap::new();
  let (board_tx, board_rx) = mpsc::channel::<Option<BoardCommunications>>();
  let (new_board_tx, new_board_rx) = mpsc::channel::<SocketAddr>();

  task!("Cloning sockets...");
  let listen_socket = home_socket.try_clone()?;
  let pulse_socket = home_socket.try_clone()?;
  pass!("Sockets cloned!");

  thread::spawn(listen(listen_socket, board_tx));
  thread::spawn(pulse(pulse_socket, new_board_rx));

  Ok(move || {
    task!("Switchboard started.");

    'a: loop {
      // interpret data from SAM board
      match board_rx.try_recv() {
        Ok(Some(BoardCommunications::Init(board_id, address))) => { 
          new_board_tx.send(address).expect("Can't find pulse for new board. This shouldn't happen.");
          
          sockets.insert(board_id.to_string(), address);

          timers.insert(board_id, 0);
        },
        Ok(Some(BoardCommunications::Sam(board_id, datapoints)))  => {
          process_sam_data(vehicle_state.clone(), mappings.clone(), board_id.clone(), datapoints);
          
          if let Some(timer) = timers.get_mut(&board_id) {
            *timer = 0;
          } else {
            warn!("Cannot find timer for board with id of {board_id}!");
          }
        },
        Ok(Some(BoardCommunications::Bsm(board_id))) => {
          warn!("Recieved BSM data from board {board_id}."); 

          if let Some(timer) = timers.get_mut(&board_id) {
            *timer = 0;
          } else {
            warn!("Cannot find timer for board with id of {board_id}!");
          }
        },
        Ok(None) => { warn!("Unknown data recieved from board!"); },
        Err(TryRecvError::Disconnected) => { warn!("Lost connection to listen() channel. This isn't supposed to happen."); },
        Err(TryRecvError::Empty) => {}
      };

      // send sam control message to SAM
      match control_rx.try_recv() {
        Ok((board_id, control_message)) => 'b: {
          let mut buf: Vec<u8> = vec![0; 1024];

          if let Err(e) = postcard::to_slice(&control_message, &mut buf) {
            fail!("postcard returned this error when attempting to serialize control message {:#?}: {e}", control_message);
            break 'b;
          }
          
          if let Some(socket) = sockets.get(&board_id) {
            match home_socket.send_to(&buf, socket) {
              Ok(size) => pass!("Sent {size} bits of control message successfully!"),
              Err(e) => fail!("Couldn't send control message to board {board_id} via socket {:#?}: {e}", socket),
            };
          } else {
            fail!("Couldn't find socket with board ID {board_id} in sockets HashMap.");
          }
        },
        Err(TryRecvError::Disconnected) => { warn!("Lost connection to control channel. This isn't supposed to happen."); },
        Err(TryRecvError::Empty) => {}
      };
      
      // update timers for all boards
      for (board_id, timer) in timers.iter_mut() {
        *timer += 1;
        if *timer > BOARD_TIMEOUT_MS {
          abort(board_id.to_string());
          break 'a;
        }
      }

      thread::sleep(Duration::from_millis(1));
    }

    fail!("Detected disconnection. Shutting down switchboard...");
  })
}

/// Constantly checks main binding for board data, handles board initalization and data encoding.
fn listen(home_socket: UdpSocket, board_tx: Sender<Option<BoardCommunications>>) -> impl FnOnce() -> () {
  move || {
    let mut buf = vec![0; 1024];
    
    let mut established_sockets = HashSet::new();

    task!("Flight Computer listening for SAM data...");
    loop {
      while let Ok((size, incoming_address)) = home_socket.recv_from(&mut buf) {
        task!("Detected datagram.");
        if size > buf.len() {
          warn!("Buffer is too small for datagram, resizing...");
          buf.resize(size, 0);
          break;
        }
        pass!("Stored within buffer.");

        task!("Interpreting buffer...");
        let raw_data = match postcard::from_bytes::<DataMessage>(&mut buf[..size]) {
          Ok(data) => data,
          Err(e) => {
            fail!("postcard couldn't interpret the datagram: {e}");
            continue;
          }
        };
        pass!("Interpreted buffer.");
  
        task!("Decoding buffer...");
        board_tx.send(match raw_data {
          DataMessage::Identity(board_id) => {
            if established_sockets.contains(&incoming_address) {
              warn!("{board_id} tried to re-establish previously established socket. Ignoring.");
              continue;
            }
            established_sockets.insert(incoming_address);

            let value = DataMessage::Identity(String::from("flight-01"));

            if let Err(e) = postcard::to_slice(&value, &mut buf) {
              warn!("postcard returned this error when attempting to serialize DataMessage::Identity: {e}");
              continue;
            }

            if let Err(e) = home_socket.send_to(&buf, incoming_address) {
              fail!("Couldn't send DataMessage::Identity to ip {incoming_address}: {e}");
            } else {
              pass!("Sent DataMessage::Identity successfully!");
            }
  
            Some(BoardCommunications::Init(board_id, incoming_address))
          },
          DataMessage::Sam(board_id, datapoints) => {
            pass!("DataMessage::Sam found!");

            Some(BoardCommunications::Sam(board_id, datapoints.to_vec()))
          },
          DataMessage::Bms(board_id) => {
            pass!("DataMessage::Bms found!");

            Some(BoardCommunications::Bsm(board_id))
          },
          _ => {
            warn!("Unknown data found.");

            None
          }
        }).expect("board_tx closed unexpectedly. This shouldn't happen.");
      }
    }
  }
}

fn pulse(socket: UdpSocket, new_board_rx: Receiver<SocketAddr>) -> impl FnOnce() -> () {
  move || {
    let mut addresses: Vec<SocketAddr> = Vec::new();
    let mut clock: u32 = 0;
    let mut buf: Vec<u8> = vec![0; 1024];

    if let Err(e) = postcard::to_slice(&DataMessage::FlightHeartbeat, &mut buf) {
      abort(format!("postcard returned this error when attempting to serialize DataMessage::FlightHeartbeat: {e}"));
      return;
    }
    
    'a: loop {
      if clock % HEARTBEAT_INTERVAL_MS == 0 {
        for address in addresses.iter() {
          if let Err(e) = socket.send_to(&buf, address) {
            abort(format!("Couldn't send heartbeat to socket {:#?}: {e}", socket));
            break 'a;
          }
        }

        clock = 0;
      }
      
      match new_board_rx.try_recv() {
        Ok(socket) => { addresses.push(socket) },
        Err(TryRecvError::Disconnected) => { warn!("Lost connection to listen() channel. This isn't supposed to happen."); },
        Err(TryRecvError::Empty) => {}
      };
  
      clock += 1;
      thread::sleep(Duration::from_millis(1));
    }
  }
}

fn abort(reason: String) {
  fail!("{}", reason);

  common::sequence::run(Sequence {
    name: "abort".to_owned(),
    script: "abort()".to_owned(),
  });
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