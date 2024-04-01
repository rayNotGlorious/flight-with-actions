mod forwarder;
mod handler;
mod state;
mod switchboard;

use std::{sync::mpsc::Sender, time::Duration};

use common::comm::{BoardId, SamControlMessage};
use jeflog::pass;
use state::ProgramState;

const SERVO_PORT: u16 = 5025;
/// Where data should be sent 
const SWITCHBOARD_ADDRESS: (&str, u16) = ("0.0.0.0", 4573);
/// SAM port to send DataMessage::Identity and DataMessage:Heartbeat to
const SAM_PORT: u16 = 8378;

/// How often heartbeats are sent
const HEARTBEAT_RATE: Duration = Duration::from_millis(50);
/// Milliseconds of inactivity before a board is declared dead
const TIME_TILL_DEATH: Duration = Duration::from_millis(50);

/// How large the buffer to send a command to a board should be (Can probably replace this with a sizeof(SamControlMessage)).
const COMMAND_MESSAGE_BUFFER_SIZE: usize = 1_024;
/// How large the buffer to recieve data from a board should be (Can probably replace this with a sizeof(DataMessage)).
const DATA_MESSAGE_BUFFER_SIZE: usize = 1_000_000;
/// How large the buffer to send a heartbeat to a board should be (Can probably replace this with a sizeof(SamControlMessage::Heartbeat)).
const HEARTBEAT_BUFFER_SIZE: usize = 1_024;


/// Board ID of the flight computer
const FC_BOARD_ID: &str = "flight-01";

type CommandSender = Sender<(BoardId, SamControlMessage)>;


fn main() {
	let mut state = ProgramState::Init;

	loop {
		pass!("Transitioned to state: {state}");
		state = state.next();
	}
}
