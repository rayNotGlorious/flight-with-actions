mod state;

use state::{State, State::*, };
use state::software_system_check::SoftwareSystemCheck;
use state::Stateful;
use crate::flight_computer::state::State::ReadLocalConfigsState;

#[derive(PartialEq, Debug)]
struct FlightComputer {
    state: State,
}

impl FlightComputer {
    pub fn new() -> FlightComputer {
        FlightComputer {
            state: SoftwareSystemCheckState(SoftwareSystemCheck {}),
        }
    }

    pub fn next_state(self) -> Self {
        let next_state = match self.state {
            SoftwareSystemCheckState(val) => val.next(),
            ReadLocalConfigsState(val) => val.next(),
            ConnectToControllerState(val) => val.next(),
            _ => UnknownState
        };

        FlightComputer {
            state: next_state
        }
    }

    // pub fn update_data(self) -> Self {
    //     return FlightComputer {
    //         state: self.state,
    //     };
    // }
}



#[cfg(test)]
mod tests {
    use crate::flight_computer::state::connect_to_controller::ConnectToController;
    use crate::flight_computer::state::read_local_configs::ReadLocalConfigs;
    use crate::flight_computer::state::software_system_check_err::SoftwareSystemCheckErr;
    use crate::flight_computer::state::spawn_cmd_receiver::SpawnCmdReceiver;
    use crate::flight_computer::state::State::{ConnectToControllerState, ReadLocalConfigsState};
    use super::*;

    /*
    Given I start the flight computer,
    Then I start in software system check
     */
    #[test]
    fn given_start_then_software_system_check() {
        let expected_state = SoftwareSystemCheckState(SoftwareSystemCheck {});

        let fc = FlightComputer::new();

        assert_eq!(expected_state, fc.state);
    }

    /*
    Given I am in software system check,
    When an error occurs while performing system check,
    Then I go to software system check err
     */
    // #[test]
    // fn given_software_system_check_when_err_then_software_system_check_err() {
    //     let expected_state = SoftwareSystemCheckErrState(SoftwareSystemCheckErr {});
    //
    //     let fc = FlightComputer { state: SoftwareSystemCheckState(SoftwareSystemCheck {}) };
    //     let fc = fc.next_state();
    //
    //     assert_eq!(expected_state, fc.state)
    // }

    /*
    Given I am in software system check,
    When the check passes,
    Then I go to read local configs
     */
    #[test]
    fn given_software_system_check_when_pass_then_read_local_configs() {
        let expected_state = ReadLocalConfigsState(ReadLocalConfigs {});
        
        let fc = FlightComputer { state: SoftwareSystemCheckState(SoftwareSystemCheck {}) };
        let fc = fc.next_state();

        assert_eq!(expected_state, fc.state)
    }

    /*
    Given I am in read local configs,
    When I complete reading local configs,
    Then I go to connect to controller
     */
    #[test]
    fn given_read_local_configs_when_done_reading_then_connect_to_controller() {
        let expected_state = ConnectToControllerState(ConnectToController {});

        let fc = FlightComputer { state: ReadLocalConfigsState(ReadLocalConfigs {}) };
        let fc = fc.next_state();

        assert_eq!(expected_state, fc.state)
    }

    /*
    Given I am in connect to controller,
    When I am connected to controller,
    Then I go to spawn command receiver
     */
    #[test]
    fn given_connect_to_controller_when_connected_then_spawn_cmd_receiver() {
        let expected_state = SpawnCmdReceiverState(SpawnCmdReceiver {});

        let fc = FlightComputer { state: ConnectToControllerState(ConnectToController {}) };
        let fc = fc.next_state();

        assert_eq!(expected_state, fc.state)
    }
}
