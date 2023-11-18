use std::net::{SocketAddr, UdpSocket};

// use fc::sequences::valve;
use fs_protobuf_rust::compiled::mcfs::{core, command, board};
use pyo3::{pyclass, pymethods};
use quick_protobuf::serialize_into_vec;
use crate::state;

#[pyclass]
#[derive(Clone, Debug)]
pub struct Valve {
	name: String,
}	

#[pymethods]
impl Valve {
	#[new]
	pub fn new(name: String) -> Self {
		Valve { name }
	}

	pub fn open(&self) {
		if let Some(valve) = state::get_valve(&self.name) {
			if let Some(hostname) = state::get_hostname_from_id(valve.board_id) {
				if let Some(ipv4_addr) = state::get_ip_from_hostname(&hostname) {
					let command = command::Command {
						command: command::mod_Command::OneOfcommand::click_valve(
							command::ClickValve { 
								valve: (Some(valve.clone())), 
								state: (board::ValveState::VALVE_OPEN)
					})};
					let message = core::Message {
						timestamp: None,
						board_id: 0,
						content: core::mod_Message::OneOfcontent::command(command)
					};
					let message_serialized = serialize_into_vec(&message).expect("Couldn't serialize message");
					let socket_addr = SocketAddr::new(ipv4_addr, 8378);
					let socket = UdpSocket::bind("0.0.0.0:9572").expect("couldn't bind to address");
                    socket.connect(socket_addr).expect("connect function failed");
                    socket.send(&message_serialized).expect("couldn't send message");
					println!("Opening valve {} on board {} at {}", self.name, hostname, ipv4_addr);
				} else {
					println!("No ip for hostname {} found", hostname);
				}
			} else {
				println!("No board with id {} found", valve.board_id);
			}
		} else {
			println!("Mapping for valve {} not found", self.name)
		}
	}

	pub fn close(&self) {
		if let Some(valve) = state::get_valve(&self.name) {
			if let Some(hostname) = state::get_hostname_from_id(valve.board_id) {
				if let Some(ipv4_addr) = state::get_ip_from_hostname(&hostname) {
					let command = command::Command {
						command: command::mod_Command::OneOfcommand::click_valve(
							command::ClickValve { 
								valve: (Some(valve.clone())), 
								state: (board::ValveState::VALVE_CLOSED)
					})};
					let message = core::Message {
						timestamp: None,
						board_id: 0,
						content: core::mod_Message::OneOfcontent::command(command)
					};
					let message_serialized = serialize_into_vec(&message).expect("Couldn't serialize message");
					let socket_addr = SocketAddr::new(ipv4_addr, 8378);
					let socket = UdpSocket::bind("0.0.0.0:9572").expect("couldn't bind to address");
                    socket.connect(socket_addr).expect("connect function failed");
                    socket.send(&message_serialized).expect("couldn't send message");
					println!("Closing valve {} on board {} at {}", self.name, hostname, ipv4_addr);
				} else {
					println!("No ip for hostname {} found", hostname);
				}
			} else {
				println!("No board with id {} found", valve.board_id);
			}
		} else {
			println!("Mapping for valve {} not found", self.name)
		}
	}

	pub fn is_open(&self) {

	}

	pub fn is_closed(&self) {

	}
}
