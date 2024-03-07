use common::comm::{ChannelType, DataMessage, DataPoint, Measurement, NodeMapping, Unit, VehicleState};
use crate::state::SharedState;
use jeflog::{fail, pass, warn};
use std::{
	collections::HashMap,
	env, 
	fs::{File, OpenOptions},
	io::{self, Write}, 
	net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket}, 
	sync::{Arc, Mutex}, 
	thread, 
	time::Duration
};

pub struct Receiver {
	vehicle_state: Arc<Mutex<VehicleState>>,
	mappings: Arc<Mutex<Vec<NodeMapping>>>,
	log_file: File,
}

impl Receiver {
    pub fn new(shared: &SharedState) -> io::Result<Self> {
        let home_dir = env::var("HOME")
            .expect("Failed to find home directory.");

		let file_path = format!("{}/bin", home_dir);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to open file: {}", e)))?;

        Ok(Receiver {
            vehicle_state: shared.vehicle_state.clone(),
            mappings: shared.mappings.clone(),
            log_file: file,
        })
    }

	pub fn receive_data(mut self) -> io::Result<impl FnOnce() -> ()> {
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
							Ok(DataMessage::Sam(data_points)) => self.process_sam_data(&socket_map, source, data_points.into_owned()),
							Ok(DataMessage::Bms) => self.process_bms_data(),
							Err(error) => {
								warn!("Failed to deserialize data message: {}.", error.to_string());
							},
						}

						let result: Result<(), io::Error> = self.log_file
							.write_all(&(buffer.len() as u32).to_le_bytes())
							.and_then(|_| {
								self.log_file.write_all(&buffer)
							});

						if let Err(error) = result {
							warn!("Failed to log data: {}", error.to_string());
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

	fn process_sam_data(&self, socket_map: &Arc<Mutex<HashMap<IpAddr, String>>>, source: SocketAddr, data_points: Vec<DataPoint>) {
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

			for data_point in data_points {
				let mapping = mappings
					.iter()
					.find(|mapping| {
						data_point.channel == mapping.channel
						&& data_point.channel_type == mapping.channel_type
						&& *board_id == mapping.board_id
					});

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
		} else {
			warn!("Received data message from unknown board at \x1b[1m{source}\x1b[0m.");
		}

		// TODO: push channel bursts into log file.
	}

	fn process_bms_data(&self) {
		warn!("Received BMS data message. Processing not currently supported.");
	}

}
