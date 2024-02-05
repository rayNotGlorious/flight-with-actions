use common::comm::{DataMessage, Measurement, NodeMapping, Unit};
use jeflog::{fail, pass, warn};
use std::{
	collections::HashMap,
	net::{IpAddr, ToSocketAddrs, UdpSocket},
	sync::{Arc, Mutex, Weak},
	thread,
	time::Duration
};

use crate::SharedState;

pub fn receive_board_data(shared: &SharedState) -> impl Fn() -> () {
	let vehicle_state = shared.vehicle_state.clone();
	let mappings = shared.mappings.clone();

	let socket = UdpSocket::bind(("0.0.0.0", 4573))
		.expect("failed to bind UDP socket.");

	let socket_map = Arc::new(Mutex::new(HashMap::new()));

	let weak_socket_map = Arc::downgrade(&socket_map);
	let discover_mappings = mappings.clone();

	thread::spawn(move || discover_data_boards(weak_socket_map, discover_mappings));

	move || {
		let mut buffer = vec![0; 1024];
			
		loop {
			match socket.recv_from(&mut buffer) {
				Ok((size, source)) => {
					if size >= buffer.capacity() {
						buffer.reserve(buffer.capacity());
						continue;
					}

					match postcard::from_bytes::<DataMessage>(&buffer[..size]) {
						Ok(DataMessage::Sam(channel_bursts)) => {
							let socket_map = socket_map
								.lock()
								.unwrap();

							if let Some(board_id) = socket_map.get(&source.ip()) {
								let mut vehicle_state = vehicle_state
									.lock()
									.unwrap();

								let mappings = mappings
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
						},
						Ok(DataMessage::Bms) => {
							warn!("Received BMS data message. Processing not currently supported.");
						}
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
	}
}

fn discover_data_boards(socket_map: Weak<Mutex<HashMap<IpAddr, String>>>, mappings: Arc<Mutex<Vec<NodeMapping>>>) {
	loop {
		if let Some(socket_map) = socket_map.upgrade() {
			let mappings = mappings
				.lock()
				.unwrap()
				.clone();

			for mapping in &mappings {
				if let Ok(mut addresses) = format!("{}.local:1", mapping.board_id).to_socket_addrs() {
					let ipv4 = addresses.find(|addr| addr.is_ipv4()).unwrap();

					pass!("Found \x1b[1m{}\x1b[0m at {ipv4}.", mapping.board_id);
					
					socket_map
						.lock()
						.unwrap()
						.insert(ipv4.ip(), mapping.board_id.clone());
				} else {
					fail!("Could not locate \x1b[1m{}\x1b[0m.", mapping.board_id);
				}
			}
		} else {
			break;
		}

		thread::sleep(Duration::from_millis(10_000));
	}
}
