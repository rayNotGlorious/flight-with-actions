use common::comm::{ChannelDataBurst, DataMessage, Measurement, NodeMapping, Unit, VehicleState};
use jeflog::{fail, pass, warn};
use std::{
	collections::HashMap,
	net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket},
	io,
	sync::{Arc, Mutex},
	thread,
	time::Duration
};

use crate::state::SharedState;

pub struct Receiver {
	vehicle_state: Arc<Mutex<VehicleState>>,
	mappings: Arc<Mutex<Vec<NodeMapping>>>,
}

impl Receiver {
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

		let discovery_handle = thread::spawn(self.discover_boards(&socket_map));

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
							Ok(DataMessage::Sam(channel_bursts)) => self.process_sam_data(&socket_map, source, channel_bursts),
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

			// when this closure finishes executing, the discover_boards thread will also
			// stop itself automatically, because the socket_map Arc will die
			if let Err(error) = discovery_handle.join() {
				warn!("Board discovery thread panicked.");
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
				let Ok(mappings) = mappings.lock() else {
					fail!("Failed to lock mappings in discover_boards.");
					return;
				};
				

				for mapping in &*mappings {
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

	fn process_sam_data(&self, socket_map: &Arc<Mutex<HashMap<IpAddr, String>>>, source: SocketAddr, channel_bursts: Vec<ChannelDataBurst>) {
		let socket_map = socket_map
			.lock()
			.unwrap();

		if let Some(board_id) = socket_map.get(&source.ip()) {
			let mut vehicle_state = self.vehicle_state
				.lock()
				.unwrap();

			let mappings = self.mappings
				.lock()
				.unwrap();

			for burst in channel_bursts {
				if let Some(last_point) = burst.data_points.last() {
					let mapping = mappings
						.iter()
						.find(|mapping| {
							burst.channel == mapping.channel && burst.channel_type == mapping.channel_type && *board_id == mapping.board_id
						});

					if let Some(mapping) = mapping {
						let measurement = Measurement {
							value: last_point.value,
							unit: Unit::Volts,
						};

						vehicle_state.sensor_readings.insert(mapping.text_id.clone(), measurement);
					} else {
						warn!("Received a data message from board {board_id}, channel {} ({:?}) with no corresponding mapping.", burst.channel, burst.channel_type);
					}
				} else {
					warn!("Received a data message from board {board_id}, channel {} ({:?}) with no data points.", burst.channel, burst.channel_type);
				}
			}
		} else {
			warn!("Received data message from unknown board at {source}: {}.", channel_bursts[0].data_points[0].value);
		}

		// TODO: push channel bursts into log file.
	}

	fn process_bms_data(&self) {
		warn!("Received BMS data message. Processing not currently supported.");
	}
}
