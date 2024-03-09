use bimap::BiHashMap;
use common::{comm::{FlightControlMessage, NodeMapping, Sequence, VehicleState}, sequence};
use jeflog::{task, pass, warn, fail};
use pyo3::Python;
use std::{io::{self, Read}, net::{IpAddr, TcpStream}, sync::{Arc, Mutex}, thread::{self, ThreadId}, time::Duration};

use crate::{forwarder, handler::create_device_handler, receiver::Receiver, SERVO_PORT};

/// Holds all shared state that should be accessible concurrently in multiple contexts.
/// 
/// Everything in this struct should be wrapped with `Arc<Mutex<T>>`. **Do not abuse this struct.**
/// It is intended for what would typically be global state.
#[derive(Debug)]
pub struct SharedState {
	pub vehicle_state: Arc<Mutex<VehicleState>>,
	pub mappings: Arc<Mutex<Vec<NodeMapping>>>,
	pub server_address: Arc<Mutex<Option<IpAddr>>>,
	pub triggers: Arc<Mutex<Vec<common::comm::Trigger>>>,
	pub sequences: Arc<Mutex<BiHashMap<String, ThreadId>>>,
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

fn init() -> ProgramState {
	let shared = SharedState {
		vehicle_state: Arc::new(Mutex::new(VehicleState::new())),
		mappings: Arc::new(Mutex::new(Vec::new())),
		server_address: Arc::new(Mutex::new(None)),
		triggers: Arc::new(Mutex::new(Vec::new())),
		sequences: Arc::new(Mutex::new(BiHashMap::new())),
	};

	sequence::initialize(shared.mappings.clone());
	sequence::set_device_handler(create_device_handler(&shared));

	let receiver = Receiver::new(&shared).expect("failed to initialize receiver");
	thread::spawn(check_triggers(&shared));

	match receiver.receive_data() {
		Ok(closure) => {
			thread::spawn(closure);
			ProgramState::ServerDiscovery { shared }
		}
		Err(error) => {
			fail!("Failed to create data forwarding closure: {error}");
			ProgramState::Init
		}
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
		let thread_id = thread::spawn(|| sequence::run(sequence))
			.thread()
			.id();

		shared.sequences
			.lock()
			.unwrap()
			.insert(sequence.name.clone(), thread_id);

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