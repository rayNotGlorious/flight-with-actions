use crate::flight_computer::state::{State, Stateful};
use crate::flight_computer::state::spawn_cmd_receiver::SpawnCmdReceiver;
use crate::flight_computer::state::State::SpawnCmdReceiverState;

#[derive(PartialEq, Debug)]
pub struct ConnectToController {}

impl Stateful for ConnectToController {
    fn next(self) -> State {
        SpawnCmdReceiverState(SpawnCmdReceiver {})
    }
}