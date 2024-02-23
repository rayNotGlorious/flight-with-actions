use common::comm::{ChannelType, DataMessage, DataPoint, Measurement, NodeMapping, Unit, VehicleState};
use crate::state::{SharedState, BoardId};
use jeflog::{pass, warn, task};
use std::{
	io::{self, ErrorKind, Read}, 
	net::{TcpStream, TcpListener}, 
	sync::{Arc, Mutex}, 
	thread, 
	time::Duration
};

pub fn start_switchboard(addr: &str, shared: &SharedState) -> io::Result<impl FnOnce() -> ()> {
	let vehicle_state = shared.vehicle_state.clone();	
	let mappings = shared.mappings.clone();
	let write_streams = shared.write_streams.clone();

	// allows data boards to bind to port
	task!("Attempting to bind to address {addr}");
	let socket = TcpListener::bind(addr)?;
	pass!("{addr} bound.");

	task!("Listening on {addr}...");
	// automatically accepts and handles valid connections
	Ok(move || {
			for stream in socket.incoming() {
				match stream {
					Ok(mut stream) => {
						task!("Incoming stream found!");
		
						let mut buffer = vec![0; 1024];
		
						match stream.read(&mut buffer) {
							Ok(size) => {
								if size == 0 {
									warn!("Connection unexpectedly closed.");
									continue;
								}

								if size >= buffer.capacity() {
									warn!("Buffer was too small for the stream data. Resizing...");
									buffer.reserve(buffer.capacity());
									continue;
								}
		
								match postcard::from_bytes::<DataMessage>(&buffer[..size]) {
									Ok(DataMessage::Identity(board_id)) => {
										let mut write_streams = write_streams.lock().unwrap();
		
										let write_stream = match stream.try_clone() {
											Ok(write_stream) => write_stream,
											Err(_) => {
												continue;
											}
										};
										let read_stream = stream;

										// at this point the primary TCP stream has been split between read and write
										write_streams.insert(board_id.clone(), write_stream);
										thread::spawn(handle_board(vehicle_state.clone(), mappings.clone(), read_stream, board_id.clone()));
										
										pass!("Data board initalization finished and thread spawned.");
									},
									Ok(_) => warn!("Initial data was not DataMessage::Identity. Ignoring..."),
									Err(error) => warn!("Failed to deserialize initalization data: {}.", error.to_string())
								}
							},
							Err(error) => {
								if let ErrorKind::Interrupted = error.kind() {
									continue;
								}
								
								warn!("Exception in reading initalization data from stream into buffer: {}.", error.to_string());
							}
						};
					},
					Err(error) => {
						warn!("Error in receiving initalization data on TCP socket: {}.", error.to_string());
						thread::sleep(Duration::from_secs(1));
					}
				}
			}
		}
	)
}

pub fn handle_board(vehicle_state: Arc<Mutex<VehicleState>>, mappings: Arc<Mutex<Vec<NodeMapping>>>, mut stream: TcpStream, board_id: BoardId) -> impl FnOnce() -> () {
	move || {
		let mut buffer = vec![0; 1024];

		loop {
			match stream.read(&mut buffer) {
				Ok(size) => {
					if size == 0 {
						warn!("Board of id {board_id} closed connection.");
						break;
					}

					if size >= buffer.capacity() {
						buffer.reserve(buffer.capacity());
						continue;
					}

					// updates vehicle state
					match postcard::from_bytes::<DataMessage>(&buffer[..size]) {
						Ok(DataMessage::Sam(data_points)) => process_sam_data(vehicle_state.clone(), mappings.clone(), board_id.clone(), data_points.into_owned()),
						Ok(DataMessage::Identity(_)) => warn!("Recieved unexpected Identity data message. Ignoring..."),
						Ok(DataMessage::Bms) => process_bms_data(),
						Err(error) => warn!("Failed to deserialize data message: {}.", error.to_string())
					}
				},
				Err(error) => {
					if let ErrorKind::Interrupted = error.kind() {
						continue;
					}
					
					warn!("Exception in reading data from stream: {}.", error.to_string());
					break;
				}
			}
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

fn process_bms_data() {
	warn!("Received BMS data message. Processing not currently supported.");
}
