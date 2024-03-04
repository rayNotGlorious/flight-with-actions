mod forwarder;
mod receiver;
mod state;
mod switchboard;

use jeflog::pass;
use state::ProgramState;

const SERVO_PORT: u16 = 5025;

fn main() {
	let mut state = ProgramState::Init;

	loop {
		pass!("Transitioned state: {:#?}", state);
		state = state.next();
	}
}
