use std::{collections::{HashMap, HashSet}, io, net::{SocketAddr, UdpSocket}, sync::{mpsc::{self, Receiver, Sender, TryRecvError}, Arc, Mutex}, thread, time::Instant};
use common::comm::{BoardId, ChannelType, CompositeValveState, DataMessage, DataPoint, Measurement, NodeMapping, SamControlMessage, SensorType, Unit, ValveState, VehicleState};
use jeflog::{task, fail, warn, pass};

use crate::{handler, state::SharedState, CommandSender};

enum BoardCommunications {
	Init(BoardId, SocketAddr),
	Sam(BoardId, Vec<DataPoint>),
	Bsm(BoardId)
}

// TODO i really need to work out what the hell to do with timers/statuses. timers can do the job of statuses, so why the am i keeping it? only issue is i need to modify timers while iterating through it.
// TODO Seperate Heartbeat and timer, don't send heartbeats to "disconnected" boards
// TODO Unobfuscate error messages
// TODO figure out what to do at the calling of each abort
// TODO Make `sockets` into a RwLock by implementing a super-loop setup: Write portion (finding all possible SAM boards), and a read portion (addressbook). (Ask jeff as RwLock implementation might avoid the need of a super-loop)

/// One-shot thread spawner, begins switchboard logic.
pub fn run(home_socket: UdpSocket, shared: SharedState) -> Result<CommandSender, io::Error> {
	let (tx, rx) = mpsc::channel::<(BoardId, SamControlMessage)>();
	thread::spawn(start_switchboard(home_socket, shared, rx)?);
	Ok(tx)
}

/// owns sockets and SharedState, changes must be sent via mpsc channel
fn start_switchboard(home_socket: UdpSocket, shared: SharedState, control_rx: Receiver<(BoardId, SamControlMessage)>) -> Result<impl FnOnce() -> (), io::Error> {
	let mappings = shared.mappings.clone();
	let vehicle_state = shared.vehicle_state.clone();

	// Boards to their to their correlated socket addresses.
	let sockets: Arc<Mutex<HashMap<BoardId, SocketAddr>>> = Arc::new(Mutex::new(HashMap::new()));

	// Boards to their heartbeat clocks (when the time hits zero, the board is considered disconnected)
	let mut timers: HashMap<BoardId, Instant> = HashMap::new();

	// Boards to their connection status
	let statuses: Arc<Mutex<HashSet<BoardId>>> = Arc::new(Mutex::new(HashSet::new()));
	
	let (board_tx, board_rx) = mpsc::channel::<Option<BoardCommunications>>();
	let listen_socket = home_socket.try_clone()?;
	let pulse_socket = home_socket.try_clone()?;

	thread::spawn(listen(listen_socket, board_tx));
	thread::spawn(pulse(&shared, pulse_socket, sockets.clone(), statuses.clone()));

	Ok(move || {
		loop {
			// interpret data from SAM board
			match board_rx.try_recv() {
				Ok(Some(BoardCommunications::Init(board_id, address))) => {
					let mut sockets = sockets.lock().unwrap();
					sockets.insert(board_id.to_string(), address);

					let mut connected = statuses.lock().unwrap();
					connected.insert(board_id.to_string());
					
					timers.insert(board_id, Instant::now());
				},
				Ok(Some(BoardCommunications::Sam(board_id, datapoints)))  => {
					process_sam_data(vehicle_state.clone(), mappings.clone(), board_id.clone(), datapoints);
					
					reset_timer(&shared, board_id, &mut timers, statuses.clone());
				},
				Ok(Some(BoardCommunications::Bsm(board_id))) => {
					warn!("Recieved BSM data from board {board_id}"); 

					reset_timer(&shared, board_id, &mut timers, statuses.clone());
				},
				Ok(None) => { warn!("Unknown data recieved from board!"); },
				Err(TryRecvError::Disconnected) => {
					warn!("Lost connection to board_tx channel. This isn't supposed to happen.");
				},
				Err(TryRecvError::Empty) => {}
			};

			// send sam control message to SAM
			match control_rx.try_recv() {
				Ok((board_id, control_message)) => 'b: {
					let mut buf = [0; crate::COMMAND_MESSAGE_BUFFER_SIZE];

					let control_message = match postcard::to_slice(&control_message, &mut buf) {
						Ok(package) =>  package,
						Err(e) => {
							fail!("postcard returned this error when attempting to serialize control message {:#?}: {e}", control_message);
							break 'b;
						}
					};
					
					let sockets = sockets.lock().unwrap();
					if let Some(socket) = sockets.get(&board_id) {
						let socket = (socket.ip(), crate::SAM_PORT);

						match home_socket.send_to(control_message, socket) {
							Ok(size) => pass!("Sent {size} bits of control message successfully!"),
							Err(e) => fail!("Couldn't send control message to board {board_id} via socket {:#?}: {e}", socket),
						};
					} else {
						fail!("Couldn't find socket with board ID {board_id} in sockets HashMap.");
					}
				},
				Err(TryRecvError::Disconnected) => { warn!("Lost connection to control_tx channel. This isn't supposed to happen."); },
				Err(TryRecvError::Empty) => {}
			};
			

			let mut statuses = statuses.lock().unwrap();
			for (board_id, timer) in timers.iter_mut() {
				if !statuses.contains(board_id) {
					continue;
				}

				if Instant::now() - *timer > crate::TIME_TILL_DEATH {
					statuses.remove(board_id);

					fail!("{}", format!("{board_id} is unresponsive. Aborting..."));
					handler::abort(&shared);
				}
			}
		}
	})
}

/// Constantly checks main binding for board data, handles board initalization and data encoding.
fn listen(home_socket: UdpSocket, board_tx: Sender<Option<BoardCommunications>>) -> impl FnOnce() -> () {
	move || {
		let mut buf = [0; crate::DATA_MESSAGE_BUFFER_SIZE];
		
		loop {
			let (size, incoming_address) = match home_socket.recv_from(&mut buf) {
				Ok(tuple) => tuple,
				Err(e) => {
					warn!("Error in receiving data from home_socket: {e}");
					continue;
				}
			};

			let raw_data = match postcard::from_bytes::<DataMessage>(&mut buf[..size]) {
				Ok(data) => data,
				Err(e) => {
					fail!("postcard couldn't interpret the datagram: {e}");
					continue;
				}
			};

			board_tx.send(match raw_data {
				DataMessage::Identity(board_id) => {
					task!("Recieved identity message from board {board_id}");
					
					let value = DataMessage::Identity(String::from(crate::FC_BOARD_ID));

					let package = match postcard::to_slice(&value, &mut buf) {
						Ok(package) => package,
						Err(e) => {
							warn!("postcard returned this error when attempting to serialize DataMessage::Identity: {e}");
							continue;
						}
					};

					if let Err(e) = home_socket.send_to(package, incoming_address) {
						fail!("Couldn't send DataMessage::Identity to ip {incoming_address}: {e}");
					} else {
						pass!("Sent DataMessage::Identity to {incoming_address} successfully.");
					}

					Some(BoardCommunications::Init(board_id, incoming_address))
				},
				DataMessage::Sam(board_id, datapoints) => Some(BoardCommunications::Sam(board_id, datapoints.to_vec())),
				DataMessage::Bms(board_id) => Some(BoardCommunications::Bsm(board_id)),
				_ => {
					warn!("Unknown data found.");
					None
				}
			}).expect("board_rx closed unexpectedly. This shouldn't happen.");	
		}
	}
}

fn pulse(shared: &SharedState, socket: UdpSocket, sockets: Arc<Mutex<HashMap<BoardId, SocketAddr>>>, statuses: Arc<Mutex<HashSet<BoardId>>>) -> impl FnOnce() -> () {
	let shared = shared.clone();
	let sockets = sockets.clone();
	let statuses = statuses.clone();

	move || {
		let mut clock: Instant = Instant::now();
		let mut buf: Vec<u8> = vec![0; crate::HEARTBEAT_BUFFER_SIZE];

		let heartbeat = match postcard::to_slice(&DataMessage::FlightHeartbeat, &mut buf) {
			Ok(package) => package,
			Err(e) => {
				fail!("postcard returned this error when attempting to serialize DataMessage::FlightHeartbeat: {e}");
				handler::abort(&shared);
				return;
			}
		};

		loop {
			if Instant::now() - clock > crate::HEARTBEAT_RATE {
				let sockets = sockets.lock().unwrap();
				let statuses = statuses.lock().unwrap();
				for (board_id, address) in sockets.iter() {
					if !statuses.contains(board_id) {
						continue;
					}

					if let Err(e) = socket.send_to(heartbeat, address) {
						fail!("Couldn't send heartbeat to socket {socket:#?}: {e}");
						handler::abort(&shared);
					}
				}

				clock = Instant::now();
			}
		}
	}
}

/// Resets the timer and connection status of the specified board.
fn reset_timer(shared: &SharedState, board_id: BoardId, timers: &mut HashMap<BoardId, Instant>, statuses: Arc<Mutex<HashSet<BoardId>>>) {
	if let Some(timer) = timers.get_mut(&board_id) {
		*timer = Instant::now();
	} else {
		fail!("Cannot find timer for board with id of {board_id}.");
		handler::abort(&shared);
	}

	let mut statuses = statuses.lock().unwrap();
	statuses.insert(board_id);
}

fn process_sam_data(vehicle_state: Arc<Mutex<VehicleState>>, mappings: Arc<Mutex<Vec<NodeMapping>>>, board_id: BoardId, data_points: Vec<DataPoint>) {
	let mut vehicle_state = vehicle_state.lock().unwrap();

	let mappings = mappings.lock().unwrap();

	for data_point in data_points {
		for mapping in &*mappings {
			// checks if this mapping corresponds to the data point and, if not, continues
			// originally, I intended to implement this with a HashMap, but considering how
			// few elements will be there, I suspect that it will actually be faster with a
			// vector and full iteration. I may be wrong; we will have to perf.
			let corresponds = data_point.channel == mapping.channel
				&& mapping.sensor_type.channel_types().contains(&data_point.channel_type)
				&& *board_id == mapping.board_id;

			if !corresponds {
				continue;
			}

			let mut text_id = mapping.text_id.clone();

			let measurement = match mapping.sensor_type {
				SensorType::RailVoltage => Measurement { value: data_point.value, unit: Unit::Volts },
				SensorType::Rtd | SensorType::Tc => Measurement { value: data_point.value, unit: Unit::Kelvin },
				SensorType::RailCurrent => Measurement { value: data_point.value, unit: Unit::Amps },
				SensorType::Pt => {
					let value;
					let unit;

					// apply linear transformations to current loop and differential signal channels
					// if the max and min are supplied by the mappings. otherwise, default back to volts.
					if let (Some(max), Some(min)) = (mapping.max, mapping.min) {
						// formula for converting voltage into psi for our PTs
						// TODO: consider precalculating scale and offset on control server
						value = (data_point.value - 0.8) / 3.2 * (max - min) + min - mapping.calibrated_offset;
						unit = Unit::Psi;
					} else {
						// if no PT ratings are set, default to displaying raw voltage
						value = data_point.value;
						unit = Unit::Volts;
					}

					Measurement { value, unit }
				},
				SensorType::LoadCell => {
					// if no load cell mappings are set, default to these values
					let mut value = data_point.value;
					let mut unit = Unit::Volts;

					// apply linear transformations to load cell channel if the max and min are supplied by the mappings.
					// otherwise, default back to volts.
					if let (Some(max), Some(min)) = (mapping.max, mapping.min) {
						// formula for converting voltage into pounds for our load cells
						value = (max - min) / 0.03 * (value + 0.015) + min - mapping.calibrated_offset;
						unit = Unit::Pounds;
					}

					Measurement { value, unit }
				},
				SensorType::Valve => {
					let voltage;
					let current;
					let measurement;

					match data_point.channel_type {
						ChannelType::ValveVoltage => {
							voltage = data_point.value;
							current = vehicle_state.sensor_readings.get(&format!("{text_id}_I"))
								.map(|measurement| measurement.value)
								.unwrap_or(0.0);

							measurement = Measurement { value: data_point.value, unit: Unit::Volts };
							text_id = format!("{text_id}_V");
						},
						ChannelType::ValveCurrent => {
							current = data_point.value;
							voltage = vehicle_state.sensor_readings.get(&format!("{text_id}_V"))
								.map(|measurement| measurement.value)
								.unwrap_or(0.0);

							measurement = Measurement { value: data_point.value, unit: Unit::Amps };
							text_id = format!("{text_id}_I");
						},
						channel_type => {
							warn!("Measured channel type of '{channel_type:?}' for valve.");
							continue;
						},
					};

					let actual_state = estimate_valve_state(voltage, current, mapping.powered_threshold, mapping.normally_closed);

					if let Some(existing) = vehicle_state.valve_states.get_mut(&mapping.text_id) {
						existing.actual = actual_state;
					} else {
						vehicle_state.valve_states.insert(mapping.text_id.clone(), CompositeValveState {
							commanded: ValveState::Undetermined,
							actual: actual_state
						});
					}

					measurement
				},
			};

			// replace item without cloning string if already present
			if let Some(existing) = vehicle_state.sensor_readings.get_mut(&text_id) {
				*existing = measurement;
			} else {
				vehicle_state.sensor_readings.insert(text_id, measurement);
			}
		}
	}
}

/// Estimates the state of a valve given its voltage, current, and the current threshold at which it is considered powered.
fn estimate_valve_state(voltage: f64, current: f64, powered_threshold: Option<f64>, normally_closed: Option<bool>) -> ValveState {
	// calculate the actual state of the valve, assuming that it's normally closed
	let mut estimated = match powered_threshold {
		Some(powered) => {
			if current < powered { // valve is unpowered
				if voltage < 4.0 {
					ValveState::Closed
				} else {
					ValveState::Disconnected
				}
			 } else { // valve is powered
				if voltage < 20.0 {
					ValveState::Fault
				} else {
					ValveState::Open
				}
			}
		},
		None => ValveState::Fault,
	};

	if normally_closed == Some(false) {
		estimated = match estimated {
			ValveState::Open => ValveState::Closed,
			ValveState::Closed => ValveState::Open,
			other => other,
		};
	}

	estimated
}
