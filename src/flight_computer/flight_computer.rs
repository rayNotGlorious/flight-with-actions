#[derive(PartialEq, Debug)]
struct FlightComputer {
    shared_data: usize,
    state: State,
}

impl FlightComputer {
    pub fn new(shared_data: usize) -> FlightComputer {
        FlightComputer {
            shared_data,
            state: State::StateA(StateA {}),
        }
    }

    pub fn next(self) -> FlightComputer {
        match self.state {
            State::StateA(val) => {
                if self.shared_data > 0 {
                    return FlightComputer {
                        shared_data: self.shared_data,
                        state: State::StateB(val.into()),
                    };
                }
            }
            _ => {}
        }
        // TODO: remove this line
        FlightComputer::new(1)
    }
}

#[derive(PartialEq, Debug)]
enum State {
    StateA(StateA),
    StateB(StateB),
}

#[derive(PartialEq, Debug)]
pub struct StateA {}

#[derive(PartialEq, Debug)]
pub struct StateB {}

impl From<StateA> for StateB {
    fn from(_prev: StateA) -> Self {
        StateB {}
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
        let flight_computer = FlightComputer::new(shared_data);
        let expected_flight_computer = FlightComputer {
            shared_data,
            state: State::StateB(StateB {}),
        };

        let flight_computer = flight_computer.next();

        assert_eq!(flight_computer, expected_flight_computer);
    }

    // Given I am in state A
    // When shared_data <= 0
    // Then go to state C

    // Given I am in state B
    // When shared_data > 10
    // Then go to state D

    // Given I am in state C
    // When shared_data <= 10
    // Then go to state D

    // Given I am in state C
    // When shared_data > 10
    // Then go to state A

    // Given I am in state D
    // When shared_data > 100
    // Then go to state A
}
