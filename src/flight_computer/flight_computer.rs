#[derive(PartialEq, Debug)]
struct FlightComputer {
    shared_data: i32,
    state: State,
}

impl FlightComputer {
    pub fn new(shared_data: i32) -> FlightComputer {
        FlightComputer {
            shared_data,
            state: State::StateA(StateA {}),
        }
    }

    pub fn next_state(self) -> Self {
        let next_state = match self.state {
            State::StateA(val) => {
                if self.shared_data > 0 {
                    State::StateB(val.into())
                } else {
                    State::StateC(val.into())
                }
            }
            State::StateB(val) => {
                if self.shared_data > 10 {
                    State::StateD(val.into())
                } else {
                    State::StateB(val.into())
                }
            }
            State::StateC(val) => {
                if self.shared_data <= 10 {
                    State::StateD(val.into())
                } else {
                    State::StateA(val.into())
                }
            }
            State::StateD(val) => {
                if self.shared_data > 100 {
                    State::StateA(val.into())
                } else {
                    State::StateD(val.into())
                }
            }
            _ => State::UnknownState,
        };

        FlightComputer {
            shared_data: self.shared_data,
            state: next_state,
        }
    }

    pub fn update_data(self) -> Self {
        return FlightComputer {
            shared_data: self.shared_data + 1,
            state: self.state,
        };
    }
}

#[derive(PartialEq, Debug)]
enum State {
    StateA(StateA),
    StateB(StateB),
    StateC(StateC),
    StateD(StateD),
    UnknownState,
}

#[derive(PartialEq, Debug)]
pub struct StateA {}

#[derive(PartialEq, Debug)]
pub struct StateB {}

#[derive(PartialEq, Debug)]
pub struct StateC {}

#[derive(PartialEq, Debug)]
pub struct StateD {}

impl From<StateA> for StateB {
    fn from(_: StateA) -> Self {
        StateB {}
    }
}

impl From<StateA> for StateC {
    fn from(_: StateA) -> Self {
        StateC {}
    }
}

impl From<StateB> for StateD {
    fn from(_: StateB) -> Self {
        StateD {}
    }
}

impl From<StateC> for StateD {
    fn from(_: StateC) -> Self {
        StateD {}
    }
}

impl From<StateC> for StateA {
    fn from(_: StateC) -> Self {
        StateA {}
    }
}

impl From<StateD> for StateA {
    fn from(_: StateD) -> Self {
        StateA {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Given I am in state A
    // When shared_data > 0
    // Then go to state B
    #[test]
    fn given_state_a_when_shared_data_gt_0_then_state_b() {
        let shared_data = 1;
        let actual_flight_computer = FlightComputer {
            shared_data,
            state: State::StateA(StateA {}),
        };
        let expected_flight_computer = FlightComputer {
            shared_data,
            state: State::StateB(StateB {}),
        };

        let actual_flight_computer = actual_flight_computer.next_state();

        assert_eq!(actual_flight_computer, expected_flight_computer);
    }

    // Given I am in state A
    // When shared_data <= 0
    // Then go to state C
    #[test]
    fn given_state_a_when_shared_data_lte_0_then_state_c() {
        let shared_data: i32 = -1;
        let actual_flight_computer = FlightComputer {
            shared_data,
            state: State::StateA(StateA {}),
        };
        let expected_flight_computer = FlightComputer {
            shared_data,
            state: State::StateC(StateC {}),
        };

        let actual_flight_computer = actual_flight_computer.next_state();

        assert_eq!(actual_flight_computer, expected_flight_computer);
    }

    // Given I am in state B
    // When shared_data > 10
    // Then go to state D
    #[test]
    fn given_state_b_when_shared_data_gt_10_then_state_d() {
        let shared_data: i32 = 11;
        let actual_flight_computer = FlightComputer {
            shared_data,
            state: State::StateB(StateB {}),
        };
        let expected_flight_computer = FlightComputer {
            shared_data,
            state: State::StateD(StateD {}),
        };

        let actual_flight_computer = actual_flight_computer.next_state();

        assert_eq!(actual_flight_computer, expected_flight_computer);
    }

    // Given I am in state C
    // When shared_data <= 10
    // Then go to state D
    #[test]
    fn given_state_c_when_shared_data_lte_10_then_state_d() {
        let shared_data: i32 = 9;
        let actual_flight_computer = FlightComputer {
            shared_data,
            state: State::StateC(StateC {}),
        };
        let expected_flight_computer = FlightComputer {
            shared_data,
            state: State::StateD(StateD {}),
        };

        let actual_flight_computer = actual_flight_computer.next_state();

        assert_eq!(actual_flight_computer, expected_flight_computer);
    }

    // Given I am in state C
    // When shared_data > 10
    // Then go to state A
    #[test]
    fn given_state_c_when_shared_data_gt_10_then_state_a() {
        let shared_data: i32 = 11;
        let actual_flight_computer = FlightComputer {
            shared_data,
            state: State::StateC(StateC {}),
        };
        let expected_flight_computer = FlightComputer {
            shared_data,
            state: State::StateA(StateA {}),
        };

        let actual_flight_computer = actual_flight_computer.next_state();

        assert_eq!(actual_flight_computer, expected_flight_computer);
    }

    // Given I am in state D
    // When shared_data > 100
    // Then go to state A
    #[test]
    fn given_state_d_when_shared_data_gt_100_then_state_a() {
        let shared_data: i32 = 101;
        let actual_flight_computer = FlightComputer {
            shared_data,
            state: State::StateD(StateD {}),
        };
        let expected_flight_computer = FlightComputer {
            shared_data,
            state: State::StateA(StateA {}),
        };

        let actual_flight_computer = actual_flight_computer.next_state();

        assert_eq!(actual_flight_computer, expected_flight_computer);
    }
}
