use common::comm::{FlightControlMessage, NodeMapping, Sequence, VehicleState};
use jeflog::{task, pass, warn, fail};
use std::{ fmt, io::{self, Read}, net::{IpAddr, TcpStream}, sync::{Arc, Mutex}, thread};

use crate::{forwarder, receiver::Receiver, SERVO_PORT};

/// Holds all shared state that should be accessible concurrently in multiple contexts.
/// 
/// Everything in this struct should be wrapped with `Arc<Mutex<T>>`. **Do not abuse this struct.**
/// It is intended for what would typically be global state.
#[derive(Debug)]
pub struct SharedState {
	pub vehicle_state: Arc<Mutex<VehicleState>>,
	pub mappings: Arc<Mutex<Vec<NodeMapping>>>,
	pub server_address: Arc<Mutex<Option<IpAddr>>>,
}

#[derive(Debug)]
pub enum ProgramState {
	/// The initialization state, which primarily spawns background threads
	/// and transitions to the `ServerDiscovery` state.
	Init,
	
	/// State which loops through potential server hostnames until locating the
	/// server and connecting to it via TCP.
	ServerDiscovery {
		/// The shared flight state.
		shared: SharedState,
	},

	/// State which waits for an operator command, such as setting mappings or
	/// running a sequence.
	WaitForOperator {
		server_socket: TcpStream,

		/// The shared flight state.
		shared: SharedState,
	},

	/// State which spawns a thread to run a sequence before returning to the
	/// `WaitForOperator` state.
	RunSequence {
		server_socket: Option<TcpStream>,

		/// A full description of the sequence to run.
		sequence: Sequence,

		/// The shared flight state.
		shared: SharedState,
	},

	/// The abort state, which safes the system and returns to `Init`.
	Abort {
		/// The shared flight state.
		shared: SharedState
	},
}

impl ProgramState {
	/// Perform transition to the next state, returning the next state. 
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

impl fmt::Display for ProgramState {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Init => write!(f, "Init"),
			Self::ServerDiscovery { .. } => write!(f, "ServerDiscovery"),
			Self::WaitForOperator { server_socket, .. } => {
				let peer_address = server_socket
					.peer_addr()
					.map(|addr| addr.to_string())
					.unwrap_or("unknown".to_owned());

				write!(f, "WaitForOperator(server = {peer_address})")
			},
			Self::RunSequence { sequence, .. } => {
				write!(f, "RunSequence(name = {})", sequence.name)
			},
			Self::Abort { .. } => write!(f, "Abort"),
		}
	}
}

fn init() -> ProgramState {
	let shared = SharedState {
		vehicle_state: Arc::new(Mutex::new(VehicleState::new())),
		mappings: Arc::new(Mutex::new(Vec::new())),
		server_address: Arc::new(Mutex::new(None)),
	};

	common::sequence::initialize(shared.vehicle_state.clone(), shared.mappings.clone());

	let receiver = Receiver::new(&shared);

	match receiver.receive_data() {
		Ok(closure) => {
			thread::spawn(closure);
			ProgramState::ServerDiscovery { shared }
		},
		Err(error) => {
			fail!("Failed to create data forwarding closure: {error}");
			ProgramState::Init
		},
	}
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
	let mut buffer = vec![0; 1_000_000];

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
