#[derive(PartialEq, Debug)]
enum FlightComputer {
    StateA(FlightComputerStateMachine<StateA>),
    StateB(FlightComputerStateMachine<StateB>),
}

impl FlightComputer {
    pub fn new(shared_data: usize) -> FlightComputer {
        FlightComputer::StateA(FlightComputerStateMachine {
            shared_data,
            state: StateA {},
        })
    }

    pub fn next(self) -> Self {
        match self {
            FlightComputer::StateA(val) => {
                if val.shared_data > 0 {
                    FlightComputer::StateB(val.into())
                } else {
                    FlightComputer::StateA(val.into())
                }
            }
            FlightComputer::StateB(val) => FlightComputer::StateB(val.into()),
        }
    }
}

#[derive(PartialEq, Debug)]
struct FlightComputerStateMachine<S> {
    shared_data: usize,
    state: S,
}

#[derive(PartialEq, Debug)]
pub struct StateA {}

#[derive(PartialEq, Debug)]
pub struct StateB {}

impl From<FlightComputerStateMachine<StateA>> for FlightComputerStateMachine<StateB> {
    fn from(prev: FlightComputerStateMachine<StateA>) -> Self {
        FlightComputerStateMachine {
            shared_data: prev.shared_data,
            state: StateB {},
        }
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
        let expected_flight_computer_state_machine = FlightComputerStateMachine {
            shared_data,
            state: StateB {},
        };

        let flight_computer = flight_computer.next();

        if let FlightComputer::StateB(flight_computer_state_machine) = flight_computer {
            assert_eq!(
                flight_computer_state_machine,
                expected_flight_computer_state_machine
            )
        }
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
