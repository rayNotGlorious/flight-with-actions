use common::comm::{ChannelType, DataMessage, DataPoint, Measurement, NodeMapping, Unit, VehicleState};
use crate::state::{SharedState, BoardId};
use jeflog::{pass, warn, task};
use std::{
	collections::HashMap, io::{self, ErrorKind, Read}, net::{TcpStream, TcpListener}, sync::{Arc, Mutex}, thread, time::Duration
};

pub struct Receiver {
	vehicle_state: Arc<Mutex<VehicleState>>,
	mappings: Arc<Mutex<Vec<NodeMapping>>>,
	write_streams: Arc<Mutex<HashMap<BoardId, Option<TcpStream>>>>
}

impl Receiver {
	pub fn new(shared: &SharedState) -> Self {
		Receiver {
			vehicle_state: shared.vehicle_state.clone(),
			mappings: shared.mappings.clone(),
			write_streams: shared.write_streams.clone()
		}
	}

	pub fn receive_data(self) -> io::Result<impl FnOnce() -> ()> {
		// allows data boards to bind to port
		let addr = "0.0.0.0:4573";
		task!("Attempting to bind to address {addr}");
		let socket = TcpListener::bind(addr)?;
		pass!("{addr} bound.");

		task!("Listening on {addr}...");
		// automatically accepts and handles valid connections
		Ok(move || {
				for read_stream in socket.incoming() {
					match read_stream {
						Ok(mut read_stream) => {
							task!("Incoming stream found!");
			
							let mut buffer = vec![0; 1024];
			
							match read_stream.read(&mut buffer) {
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
											let write_steams = self.write_streams.clone();
											let mut write_streams = write_steams.lock().unwrap();
			
											// supposed to "clone" the current read stream into a write stream
											// there is no difference between a read and write stream, we just call them that as a convention
											write_streams.insert(board_id.clone(), 
												if let Ok(address) = read_stream.local_addr() {
													TcpStream::connect(address).ok()
												} else {
													None
												}
											);
			
											thread::spawn(self.listen_for_board_data(read_stream, board_id.clone()));
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
			})
	}

	pub fn listen_for_board_data(self, mut stream: TcpStream, board_id: BoardId) -> impl FnOnce() -> () {
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
							Ok(DataMessage::Sam(data_points)) => self.process_sam_data(board_id.clone(), data_points.into_owned()),
							Ok(DataMessage::Identity(_)) => warn!("Recieved unexpected Identity data message. Ignoring..."),
							Ok(DataMessage::Bms) => self.process_bms_data(),
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

	fn process_sam_data(&self, board_id: BoardId, data_points: Vec<DataPoint>) {
		let mut vehicle_state = self.vehicle_state.lock().unwrap();

		let mappings = self.mappings.lock().unwrap();

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

	fn process_bms_data(&self) {
		warn!("Received BMS data message. Processing not currently supported.");
	}
}


