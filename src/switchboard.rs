use std::{collections::{HashMap, HashSet}, io, net::{SocketAddr, UdpSocket}, sync::{mpsc::{self, Receiver, Sender, TryRecvError}, Arc, Mutex}, thread, time::{Duration, Instant}};
use common::comm::{BoardId, ChannelType, CompositeValveState, DataMessage, DataPoint, Measurement, NodeMapping, SamControlMessage, SensorType, Unit, ValveState, VehicleState};
use jeflog::{task, fail, warn, pass};

use crate::{handler, state::SharedState, CommandSender};

/// Milliseconds of inactivity before we sent a heartbeat
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(50);

enum BoardCommunications {
	Init(BoardId, SocketAddr),
	Sam(BoardId, Vec<DataPoint>),
	Bsm(BoardId)
}

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

	let sockets: Arc<Mutex<HashMap<BoardId, SocketAddr>>> = Arc::new(Mutex::new(HashMap::new()));
	let mut timers: HashMap<BoardId, Option<Instant>> = HashMap::new();
	let (board_tx, board_rx) = mpsc::channel::<Option<BoardCommunications>>();
	
	task!("Cloning sockets...");
	let listen_socket = home_socket.try_clone()?;
	let pulse_socket = home_socket.try_clone()?;
	pass!("Sockets cloned!");

	thread::spawn(listen(listen_socket, board_tx));
	thread::spawn(pulse(pulse_socket, sockets.clone(), &shared));

	Ok(move || {
		task!("Switchboard started.");
    
		loop {
			// interpret data from SAM board
			match board_rx.try_recv() {
				Ok(Some(BoardCommunications::Init(board_id, address))) => {
					let mut sockets = sockets.lock().unwrap();
					sockets.insert(board_id.to_string(), address);

					timers.insert(board_id, Some(Instant::now()));
				},
				Ok(Some(BoardCommunications::Sam(board_id, datapoints)))  => {
					process_sam_data(vehicle_state.clone(), mappings.clone(), board_id.clone(), datapoints);
					
					if let Some(timer) = timers.get_mut(&board_id) {
						*timer = Some(Instant::now());
					} else {
						warn!("Cannot find timer for board with id of {board_id}!");
					}
				},
				Ok(Some(BoardCommunications::Bsm(board_id))) => {
					warn!("Recieved BSM data from board {board_id}"); 

					if let Some(timer) = timers.get_mut(&board_id) {
						*timer = Some(Instant::now());
					} else {
						warn!("Cannot find timer for board with id of {board_id}!");
					}
				},
				Ok(None) => { warn!("Unknown data recieved from board!"); },
				Err(TryRecvError::Disconnected) => { warn!("Lost connection to board_tx channel. This isn't supposed to happen."); },
				Err(TryRecvError::Empty) => {}
			};

			// send sam control message to SAM
			match control_rx.try_recv() {
				Ok((board_id, control_message)) => 'b: {
					let mut buf = [0; 1024];

					let control_message = match postcard::to_slice(&control_message, &mut buf) {
						Ok(package) =>  package,
						Err(e) => {
							fail!("postcard returned this error when attempting to serialize control message {:#?}: {e}", control_message);
							break 'b;
						}
					};
					
					let sockets = sockets.lock().unwrap();
					if let Some(socket) = sockets.get(&board_id) {
						let socket = (socket.ip(), 8378);

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
			

			// make this cleaner
			// update timers for all boards
			for (board_id, timer) in timers.iter_mut() {
				if let Some(raw_time) = timer {
					if Instant::now() - *raw_time > HEARTBEAT_INTERVAL {
						fail!("{}", format!("{board_id} is unresponsive. Aborting..."));
						abort(&shared);
						*timer = None;
					}
				}
			}
		}
	})
}

/// Constantly checks main binding for board data, handles board initalization and data encoding.
fn listen(home_socket: UdpSocket, board_tx: Sender<Option<BoardCommunications>>) -> impl FnOnce() -> () {
	move || {
		let mut buf = [0; 1_000_000];
		
		let mut established_sockets = HashSet::new();

		task!("Flight Computer listening for SAM data...");
		loop {
			let (size, incoming_address) = match home_socket.recv_from(&mut buf) {
				Ok(tuple) => tuple,
				Err(e) => {
					warn!("Error in receiving data from home_socket: {e}");
					continue;
				}
			};

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
						warn!("{board_id} sent an Identity after it already was sent one.");
					} else {
						established_sockets.insert(incoming_address);
					}
					
					let value = DataMessage::Identity(String::from("flight-01"));

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
						pass!("Sent DataMessage::Identity successfully.");
					}

					Some(BoardCommunications::Init(board_id, incoming_address))
				},
				DataMessage::Sam(board_id, datapoints) => {
					pass!("Received DataMessage::Sam from {board_id}");

					Some(BoardCommunications::Sam(board_id, datapoints.to_vec()))
				},
				DataMessage::Bms(board_id) => {
					pass!("Received DataMessage::Bms from {board_id}");

					Some(BoardCommunications::Bsm(board_id))
				},
				_ => {
					warn!("Unknown data found.");

					None
				}
			}).expect("board_rx closed unexpectedly. This shouldn't happen.");	
		}
	}
}

fn pulse(socket: UdpSocket, sockets: Arc<Mutex<HashMap<BoardId, SocketAddr>>>, shared: &SharedState) -> impl FnOnce() -> () {
	let shared = shared.clone();
	let sockets = sockets.clone();

	move || {
		let mut clock: Instant = Instant::now();
		let mut buf: Vec<u8> = vec![0; 1024];

		let heartbeat = match postcard::to_slice(&DataMessage::FlightHeartbeat, &mut buf) {
			Ok(package) => package,
			Err(e) => {
				fail!("postcard returned this error when attempting to serialize DataMessage::FlightHeartbeat: {e}");
				abort(&shared);
				return;
			}
		};

		loop {
			if Instant::now() - clock > HEARTBEAT_INTERVAL {
				let sockets = sockets.lock().unwrap();
				for address in sockets.iter() {
					if let Err(e) = socket.send_to(heartbeat, address.1) {
						fail!("Couldn't send heartbeat to socket {socket:#?}: {e}");
						abort(&shared);
					}
				}

				clock = Instant::now();
			}
		}
	}
}

fn abort(shared: &SharedState) {
	let sequences = shared.sequences.lock().unwrap();

	if sequences.is_empty() {
		fail!("No sequence to abort!");
		return;
	}

	handler::abort(shared);
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
