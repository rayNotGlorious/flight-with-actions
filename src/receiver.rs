use common::comm::{ChannelType, CompositeValveState, DataMessage, DataPoint, Measurement, NodeMapping, SensorType, Unit, ValveState, VehicleState};
use crate::state::SharedState;
use jeflog::{fail, pass, warn};
use std::{
	collections::HashMap, io, net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket}, sync::{Arc, Mutex}, thread, time::Duration
};

/// Receives and processes data from SAM boards before putting it into the vehicle state.
#[derive(Clone, Debug)]
pub struct Receiver {
	vehicle_state: Arc<Mutex<VehicleState>>,
	mappings: Arc<Mutex<Vec<NodeMapping>>>,
}

impl Receiver {
	/// Constructs a new `Receiver` by cloning necessary references in `SharedState`.
	pub fn new(shared: &SharedState) -> Self {
		Receiver {
			vehicle_state: shared.vehicle_state.clone(),
			mappings: shared.mappings.clone(),
		}
	}

	pub fn receive_data(self) -> io::Result<impl FnOnce() -> ()> {
		let socket_map = Arc::new(Mutex::new(HashMap::new()));

		// TODO: change this when switching to TCP
		let socket = UdpSocket::bind(("0.0.0.0", 4573))?;

		thread::spawn(self.discover_boards(&socket_map));

		Ok(move || {
			let mut buffer = vec![0; 1024];
				
			loop {
				match socket.recv_from(&mut buffer) {
					Ok((size, source)) => {
						if size >= buffer.capacity() {
							buffer.reserve(buffer.capacity());
							continue;
						}

						match postcard::from_bytes::<DataMessage>(&buffer[..size]) {
							Ok(DataMessage::Sam(data_points)) => self.process_sam_data(&socket_map, source, data_points.into_owned()),
							Ok(DataMessage::Bms) => self.process_bms_data(),
							Err(error) => {
								warn!("Failed to deserialize data message: {}.", error.to_string());
							},
						}
					},
					Err(error) => {
						warn!("Failed to receive data on UDP socket: {}.", error.to_string());
						thread::sleep(Duration::from_secs(1));
					},
				}
			}
		})
	}

	fn discover_boards(&self, socket_map: &Arc<Mutex<HashMap<IpAddr, String>>>) -> impl Fn() -> () {
		// since socket_map is tied to its strong counterpart in receive_data, the returned closure will
		// automatically return whenever receive_data returns
		let socket_map = Arc::downgrade(socket_map);
		let mappings = self.mappings.clone();

		move || {
			while let Some(socket_map) = socket_map.upgrade() {
				let mappings = mappings.lock().unwrap().clone();

				for mapping in mappings {
					if mapping.board_id == "localhost" {
						let Ok(mut socket_map) = socket_map.lock() else {
							fail!("Failed to lock socket map in discover_boards.");
							return;
						};

						socket_map.insert(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), mapping.board_id.clone());

						continue;
					}

					if let Ok(mut addresses) = format!("{}.local:1", mapping.board_id).to_socket_addrs() {
						let ipv4 = addresses.find(|addr| addr.is_ipv4()).unwrap();
	
						pass!("Found \x1b[1m{}\x1b[0m at {ipv4}.", mapping.board_id);
						
						let Ok(mut socket_map) = socket_map.lock() else {
							fail!("Failed to lock socket map in discover_boards.");
							return;
						};

						socket_map.insert(ipv4.ip(), mapping.board_id.clone());
					} else {
						fail!("Could not locate \x1b[1m{}\x1b[0m.", mapping.board_id);
					}
				}

				thread::sleep(Duration::from_millis(10_000));
			}
		}
	}

	fn process_sam_data(&self, socket_map: &Arc<Mutex<HashMap<IpAddr, String>>>, source: SocketAddr, data_points: Vec<DataPoint>) {
		let socket_map = socket_map
			.lock()
			.unwrap();

		let Some(board_id) = socket_map.get(&source.ip()) else {
			warn!("Received data message from unknown board at \x1b[1m{source}\x1b[0m.");
			return;
		};

		let mut vehicle_state = self.vehicle_state
			.lock()
			.unwrap();

		let mappings = self.mappings
			.lock()
			.unwrap();

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
					SensorType::LoadCell | SensorType::RailVoltage => Measurement { value: data_point.value, unit: Unit::Volts },
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

	fn process_bms_data(&self) {
		warn!("Received BMS data message. Processing not currently supported.");
	}
}

/// Estimates the state of a valve given its voltage, current, and the current threshold at which it is considered powered.
pub fn estimate_valve_state(voltage: f64, current: f64, powered_threshold: Option<f64>, normally_closed: Option<bool>) -> ValveState {
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
