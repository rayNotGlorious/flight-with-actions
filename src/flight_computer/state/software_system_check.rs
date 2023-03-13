use crate::flight_computer::state::{State, State::SoftwareSystemCheckErrState, Stateful};
use crate::flight_computer::state::read_local_configs::ReadLocalConfigs;
use crate::flight_computer::state::software_system_check_err::SoftwareSystemCheckErr;
use crate::flight_computer::state::State::ReadLocalConfigsState;

#[derive(PartialEq, Debug)]
pub struct SoftwareSystemCheck {}

impl Stateful for SoftwareSystemCheck {
    fn next(self) -> State {
        ReadLocalConfigsState(ReadLocalConfigs {})
    }
}
