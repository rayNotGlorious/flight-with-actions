mod forwarder;
mod handler;
mod state;
mod switchboard;

use std::sync::mpsc::Sender;

use common::comm::{BoardId, SamControlMessage};
use jeflog::pass;
use state::ProgramState;

const SERVO_PORT: u16 = 5025;
type CommandSender = Sender<(BoardId, SamControlMessage)>;

fn main() {
	let mut state = ProgramState::Init;

	loop {
		pass!("Transitioned to state: {state}");
		state = state.next();
	}
}
