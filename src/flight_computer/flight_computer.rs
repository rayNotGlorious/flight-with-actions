#[derive(PartialEq, Debug)]
pub struct FlightComputer<S> {
    shared_value_between_states: usize,
    state: S,
}

impl FlightComputer<StateA> {
    pub fn new(shared_value_between_states: usize) -> Self {
        FlightComputer {
            shared_value_between_states,
            state: StateA {},
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct StateA {}

impl From<FlightComputer<StateC>> for FlightComputer<StateA> {
    fn from(prev_state: FlightComputer<StateC>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateA {},
        }
    }
}

impl From<FlightComputer<StateD>> for FlightComputer<StateA> {
    fn from(prev_state: FlightComputer<StateD>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateA {},
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct StateB {}

impl From<FlightComputer<StateA>> for FlightComputer<StateB> {
    fn from(prev_state: FlightComputer<StateA>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateB {},
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct StateC {}

impl From<FlightComputer<StateA>> for FlightComputer<StateC> {
    fn from(prev_state: FlightComputer<StateA>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateC {},
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct StateD {}

impl From<FlightComputer<StateB>> for FlightComputer<StateD> {
    fn from(prev_state: FlightComputer<StateB>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateD {},
        }
    }
}

impl From<FlightComputer<StateC>> for FlightComputer<StateD> {
    fn from(prev_state: FlightComputer<StateC>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateD {},
        }
    }
}

#[cfg(test)]
mod tests {
}
