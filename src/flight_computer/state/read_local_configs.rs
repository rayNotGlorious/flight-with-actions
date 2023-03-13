use crate::flight_computer::state::{
    connect_to_controller::ConnectToController, State, State::ConnectToControllerState, Stateful,
};

#[derive(PartialEq, Debug)]
pub struct ReadLocalConfigs {}

impl Stateful for ReadLocalConfigs {
    fn next(self) -> State {
        ConnectToControllerState(ConnectToController {})
    }
}
