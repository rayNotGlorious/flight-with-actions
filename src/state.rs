use common::comm::{FlightControlMessage, NodeMapping, Sequence, VehicleState, BoardId};
use jeflog::{task, pass, warn, fail};
use std::{io::{self, Read}, net::{IpAddr, TcpStream, UdpSocket}, sync::{mpsc::Sender, Arc, Mutex}, thread};

use crate::{forwarder, switchboard, SERVO_PORT};

/// Holds all shared state that should be accessible concurrently in multiple contexts.
/// 
/// Everything in this struct should be wrapped with `Arc<Mutex<T>>`. **Do not abuse this struct.**
/// It is intended for what would typically be global state.
#[derive(Debug)]
pub struct SharedState {
	pub vehicle_state: Arc<Mutex<VehicleState>>,
	pub mappings: Arc<Mutex<Vec<NodeMapping>>>,
	pub server_address: Arc<Mutex<Option<IpAddr>>>,
	pub sequence_tx: Sender<(BoardId, Sequence)>
}

#[derive(Debug)]
pub enum ProgramState {
	Init,
	ServerDiscovery {
		shared: SharedState,
	},
	WaitForOperator {
		server_socket: TcpStream,

		shared: SharedState,
	},
	RunSequence {
		server_socket: Option<TcpStream>,
		sequence: Sequence,

		shared: SharedState,
	},
	Abort {
		shared: SharedState
	},
}

impl ProgramState {
	pub fn next(self) -> Self {
		match self {
			ProgramState::Init => init(),
			ProgramState::ServerDiscovery { shared } => server_discovery(shared),
			ProgramState::WaitForOperator { server_socket, shared } => wait_for_operator(server_socket, shared),
			ProgramState::RunSequence { server_socket, sequence, shared } => run_sequence(server_socket, sequence, shared),
			ProgramState::Abort { shared } => abort(shared),
		}
	}
}

const BIND_ADDRESS: (&str, u16) = ("0.0.0.0", 4573);

fn init() -> ProgramState {
	let home_socket = UdpSocket::bind(BIND_ADDRESS)
		.expect(&format!("Cannot create bind on port {:#?}", BIND_ADDRESS));
	let vehicle_state = Arc::new(Mutex::new(VehicleState::new()));
	let mappings: Arc<Mutex<Vec<NodeMapping>>> = Arc::new(Mutex::new(Vec::new()));
	let sequence_tx = switchboard::run(home_socket, mappings.clone(), vehicle_state.clone())
		.expect("Couldn't start switchboard.");
	
	let shared = SharedState {
		vehicle_state,
		mappings,
		server_address: Arc::new(Mutex::new(None)),
		sequence_tx
	};

	common::sequence::initialize(shared.vehicle_state.clone(), shared.mappings.clone());

	ProgramState::ServerDiscovery { shared }
}

fn server_discovery(shared: SharedState) -> ProgramState {
	task!("Locating control server.");

	let potential_hostnames = ["server-01.local", "server-02.local", "localhost"];

	for host in potential_hostnames {
		task!("Attempting to connect to \x1b[1m{}:{SERVO_PORT}\x1b[0m.", host);

		if let Ok(stream) = TcpStream::connect((host, SERVO_PORT)) {
			pass!("Successfully connected to \x1b[1m{}:{SERVO_PORT}\x1b[0m.", host);
			pass!("Found control server at \x1b[1m{}:{SERVO_PORT}\x1b[0m.", host);

			*shared.server_address.lock().unwrap() = Some(stream.peer_addr().unwrap().ip());
			thread::spawn(forwarder::forward_vehicle_state(&shared));

			return ProgramState::WaitForOperator { server_socket: stream, shared };
		}

		fail!("Failed to connect to \x1b[1m{}:{SERVO_PORT}\x1b[0m.", host);
	}

	fail!("Failed to locate control server at all potential hostnames. Retrying.");
	ProgramState::ServerDiscovery { shared }
}

fn wait_for_operator(mut server_socket: TcpStream, shared: SharedState) -> ProgramState {
	let mut buffer = vec![0; 1024];

	match server_socket.read(&mut buffer) {
		Ok(size) => {
			// if the size is zero, a TCP shutdown packet was sent. the connection is closed.
			if size == 0 {
				return ProgramState::ServerDiscovery { shared };
			}

			match postcard::from_bytes::<FlightControlMessage>(&buffer) {
				Ok(message) => {
					match message {
						FlightControlMessage::Mappings(mappings) => {
							pass!("Received mappings from server: {mappings:#?}");
							*shared.mappings.lock().unwrap() = mappings;
							ProgramState::WaitForOperator { server_socket, shared }
						},
						FlightControlMessage::Sequence(sequence) => {
							pass!("Received sequence from server: {sequence:#?}");
							ProgramState::RunSequence {
								server_socket: Some(server_socket),
								sequence,
								shared,
							}
						},
						FlightControlMessage::Trigger(_) => {
							warn!("Received control message setting trigger. Triggers not yet supported.");
							ProgramState::WaitForOperator { server_socket, shared }
						},
					}
				},
				Err(error) => {
					warn!("Failed to deserialize control message: {}.", error.to_string());
					ProgramState::WaitForOperator { server_socket, shared }
				}
			}
		},
		Err(ref error) if error.kind() == io::ErrorKind::WouldBlock => {
			ProgramState::WaitForOperator { server_socket, shared }
		},
		Err(error) => {
			fail!("Failed to read from server socket: {}. Dropping connection.", error.to_string());
			ProgramState::ServerDiscovery { shared }
		}
	}
}

fn run_sequence(server_socket: Option<TcpStream>, sequence: Sequence, shared: SharedState) -> ProgramState {
	common::sequence::run(sequence);

	// differentiates between an abort sequence and a normal sequence.
	// abort does not have access to the server socket, so it gives None for it.
	// if an abort is run, then we need to return to ServerDiscovery to reconnect.
	if let Some(server_socket) = server_socket {
		ProgramState::WaitForOperator { server_socket, shared }
	} else {
		ProgramState::ServerDiscovery { shared }
	}
}

fn abort(shared: SharedState) -> ProgramState {
	ProgramState::RunSequence {
		sequence: Sequence {
			name: "abort".to_owned(),
			script: "abort()".to_owned(),
		},
		server_socket: None,
		shared,
	}
}