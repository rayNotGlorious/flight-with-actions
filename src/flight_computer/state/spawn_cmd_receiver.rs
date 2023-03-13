use crate::flight_computer::state::{State, Stateful};

#[derive(PartialEq, Debug)]
pub struct SpawnCmdReceiver {}

impl Stateful for SpawnCmdReceiver {
    fn next(self) -> State {
        todo!()
    }
}