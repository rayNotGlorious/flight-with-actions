#![warn(missing_docs)]

//! Flight computer software.

/// Holds all components related to forwarding data out to the control server.
pub mod forwarder;

/// Holds all components related to receiving data from vehicle/ground boards.
pub mod receiver;

/// Holds all components related the primary state machine.
pub mod state;

use jeflog::pass;
use state::ProgramState;

const SERVO_PORT: u16 = 5025;

fn main() {
	let mut state = ProgramState::Init;

	loop {
		pass!("Transitioned to state: {state}");
		state = state.next();
	}
}
