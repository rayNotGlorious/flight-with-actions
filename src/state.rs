use common::{comm::{FlightControlMessage, SamControlMessage, NodeMapping, Sequence, VehicleState, BoardId}, sequence};
use jeflog::{task, pass, warn, fail};
use std::{fmt, time::Duration, io::{self, Read}, net::{IpAddr, TcpStream, UdpSocket}, sync::{mpsc::Sender, Arc, Mutex}, thread::{self, ThreadId}};
use bimap::BiHashMap;
use crate::{forwarder, switchboard, handler::create_device_handler, SERVO_PORT};
use pyo3::Python;

/// Holds all shared state that should be accessible concurrently in multiple contexts.
/// 
/// Everything in this struct should be wrapped with `Arc<Mutex<T>>`. **Do not abuse this struct.**
/// It is intended for what would typically be global state.
#[derive(Debug)]
pub struct SharedState {
	pub vehicle_state: Arc<Mutex<VehicleState>>,
	pub mappings: Arc<Mutex<Vec<NodeMapping>>>,
	pub server_address: Arc<Mutex<Option<IpAddr>>>,
	pub command_tx: Sender<(BoardId, SamControlMessage)>,
	pub triggers: Arc<Mutex<Vec<common::comm::Trigger>>>,
	pub sequences: Arc<Mutex<BiHashMap<String, ThreadId>>>,
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

const BIND_ADDRESS: (&str, u16) = ("0.0.0.0", 4573);
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
	let home_socket = UdpSocket::bind(BIND_ADDRESS)
		.expect(&format!("Cannot create bind on port {:#?}", BIND_ADDRESS));
	let vehicle_state = Arc::new(Mutex::new(VehicleState::new()));
	let mappings: Arc<Mutex<Vec<NodeMapping>>> = Arc::new(Mutex::new(Vec::new()));
	let command_tx = 
		match switchboard::run(home_socket, mappings.clone(), vehicle_state.clone()) {
			Ok(command_tx) => command_tx,
			Err(error) => {
				fail!("Failed to create switchboard: {error}");
				return ProgramState::Init;
			}
	};
	
	let shared = SharedState {
		vehicle_state,
		mappings,
		server_address: Arc::new(Mutex::new(None)),
		command_tx,
		triggers: Arc::new(Mutex::new(Vec::new())),
		sequences: Arc::new(Mutex::new(BiHashMap::new())),
	};

	sequence::initialize(shared.mappings.clone());
	sequence::set_device_handler(create_device_handler(&shared));

	thread::spawn(check_triggers(&shared));

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
						FlightControlMessage::Trigger(trigger) => {
							pass!("Received trigger from server: {trigger:#?}");
							
							// update existing trigger if one has the same name
							// otherwise, add a new trigger to the vec
							let mut triggers = shared.triggers.lock().unwrap();

							let existing = triggers
								.iter()
								.position(|t| t.name == trigger.name);

							if let Some(index) = existing {
								triggers[index] = trigger;
							} else {
								triggers.push(trigger);
							}

							// necessary to allow passing 'shared' back to WaitForOperator
							drop(triggers);

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
	if let Some(server_socket) = server_socket {
		let sequence_name = sequence.name.clone();

		let thread_id = thread::spawn(|| sequence::run(sequence))
			.thread()
			.id();

		shared.sequences
			.lock()
			.unwrap()
			.insert(sequence_name, thread_id);

		ProgramState::WaitForOperator { server_socket, shared }
	} else {
		shared.sequences
			.lock()
			.unwrap()
			.clear();

		sequence::run(sequence);

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

fn check_triggers(shared: &SharedState) -> impl FnOnce() -> () {
	let triggers = shared.triggers.clone();

	// return closure instead of using the function itself because of borrow-checking
	// rules regarding moving the 'triggers' reference across closure bounds
	move || {
		loop {
			let mut triggers = triggers.lock().unwrap();

			for trigger in triggers.iter_mut() {
				// perform check by running condition as Python script and getting truth value
				let check = Python::with_gil(|py| {
					py.eval(&trigger.condition, None, None)
						.and_then(|condition| {
							condition.extract::<bool>()
						})
				});

				// checks if the condition evaluated true
				if check.as_ref().is_ok_and(|c| *c) {
					let sequence = Sequence {
						name: format!("trigger_{}", trigger.name),
						script: trigger.script.clone(),
					};

					// run sequence in the same thread so there is no rapid-fire
					// sequence dispatches if a trigger is tripped
					// note: this is intentionally blocking
					common::sequence::run(sequence);
				}

				if let Err(error) = check {
					fail!("Trigger '{}' raised exception during execution: {error}", trigger.name);
					trigger.active = false;
				}
			}

			// drop triggers before waiting so the lock isn't held over the wait
			drop(triggers);
			thread::sleep(Duration::from_millis(10));
		}
	}
}