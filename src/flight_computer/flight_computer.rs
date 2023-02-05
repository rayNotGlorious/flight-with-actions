#[derive(PartialEq, Debug)]
pub struct FlightComputer<S> {
    shared_value_between_states: usize,
    state: S,
}

const DEFAULT_VALUE: usize = 10;

impl FlightComputer<StateA> {
    pub fn new(shared_value_between_states: usize) -> Self {
        FlightComputer {
            shared_value_between_states,
            state: StateA {
                a_value: DEFAULT_VALUE,
            },
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct StateA {
    a_value: usize,
}

impl From<FlightComputer<StateC>> for FlightComputer<StateA> {
    fn from(prev_state: FlightComputer<StateC>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateA { a_value: 0 },
        }
    }
}

impl From<FlightComputer<StateD>> for FlightComputer<StateA> {
    fn from(prev_state: FlightComputer<StateD>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateA { a_value: 0 },
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct StateB {
    b_value: usize,
}

impl From<FlightComputer<StateA>> for FlightComputer<StateB> {
    fn from(prev_state: FlightComputer<StateA>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateB { b_value: 20 },
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct StateC {
    c_value: usize,
}

impl From<FlightComputer<StateA>> for FlightComputer<StateC> {
    fn from(prev_state: FlightComputer<StateA>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateC { c_value: 30 },
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct StateD {
    d_value: usize,
}

impl From<FlightComputer<StateB>> for FlightComputer<StateD> {
    fn from(prev_state: FlightComputer<StateB>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateD { d_value: 40 },
        }
    }
}

impl From<FlightComputer<StateC>> for FlightComputer<StateD> {
    fn from(prev_state: FlightComputer<StateC>) -> Self {
        FlightComputer {
            shared_value_between_states: prev_state.shared_value_between_states,
            state: StateD { d_value: 40 },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::flight_computer::flight_computer::{FlightComputer, StateA, StateB, StateC, StateD};

    // test A can go to B
    #[test]
    fn test_a_can_go_to_b() {
        let expected_shared_value_between_states = 10;
        let expected_b_value = 20;
        let expected_state = StateB {
            b_value: expected_b_value,
        };

        let in_state_a = FlightComputer::<StateA>::new(expected_shared_value_between_states);
        let in_state_b: FlightComputer<StateB> = in_state_a.into();

        assert_eq!(
            expected_shared_value_between_states,
            in_state_b.shared_value_between_states
        );
        assert_eq!(expected_state, in_state_b.state);
    }

    // test A can go to C
    #[test]
    fn test_a_can_go_to_c() {
        let expected_shared_value_between_states = 10;
        let expected_c_value = 30;
        let expected_state = StateC {
            c_value: expected_c_value,
        };

        let in_state_a = FlightComputer::<StateA>::new(expected_shared_value_between_states);
        let in_state_c: FlightComputer<StateC> = in_state_a.into();

        assert_eq!(
            expected_shared_value_between_states,
            in_state_c.shared_value_between_states
        );
        assert_eq!(expected_state, in_state_c.state);
    }

    // test B can go to D
    #[test]
    fn test_b_can_go_to_d() {
        let expected_shared_value_between_states = 10;
        let expected_d_value = 40;
        let expected_state = StateD {
            d_value: expected_d_value,
        };

        let in_state_a = FlightComputer::<StateA>::new(expected_shared_value_between_states);
        let in_state_b: FlightComputer<StateB> = in_state_a.into();
        let in_state_d: FlightComputer<StateD> = in_state_b.into();

        assert_eq!(
            expected_shared_value_between_states,
            in_state_d.shared_value_between_states
        );
        assert_eq!(expected_state, in_state_d.state);
    }

    // test C can go to D
    #[test]
    fn test_c_can_go_to_d() {
        let expected_shared_value_between_states = 10;
        let expected_d_value = 40;
        let expected_state = StateD {
            d_value: expected_d_value,
        };

        let in_state_a = FlightComputer::<StateA>::new(expected_shared_value_between_states);
        let in_state_c: FlightComputer<StateC> = in_state_a.into();
        let in_state_d: FlightComputer<StateD> = in_state_c.into();

        assert_eq!(
            expected_shared_value_between_states,
            in_state_d.shared_value_between_states
        );
        assert_eq!(expected_state, in_state_d.state);
    }

    // test C can go to A
    #[test]
    fn test_c_can_go_to_a() {
        let expected_shared_value_between_states = 10;
        let expected_a_value = 0;
        let expected_state = StateA {
            a_value: expected_a_value,
        };

        let in_state_a = FlightComputer::<StateA>::new(expected_shared_value_between_states);
        let in_state_c: FlightComputer<StateC> = in_state_a.into();
        let in_state_a: FlightComputer<StateA> = in_state_c.into();

        assert_eq!(
            expected_shared_value_between_states,
            in_state_a.shared_value_between_states
        );
        assert_eq!(expected_state, in_state_a.state);
    }

    // test D can go to A
    #[test]
    fn test_d_can_go_to_a() {
        let expected_shared_value_between_states = 10;
        let expected_a_value = 0;
        let expected_state = StateA {
            a_value: expected_a_value,
        };

        let in_state_a = FlightComputer::<StateA>::new(expected_shared_value_between_states);
        let in_state_c: FlightComputer<StateC> = in_state_a.into();
        let in_state_d: FlightComputer<StateD> = in_state_c.into();
        let in_state_a: FlightComputer<StateA> = in_state_d.into();

        assert_eq!(
            expected_shared_value_between_states,
            in_state_a.shared_value_between_states
        );
        assert_eq!(expected_state, in_state_a.state);
    }

    // test A can't go to D
    #[test]
    fn test_a_cant_go_to_d() {
        /* The following code does not compile!

        let expected_shared_value_between_states = 10;
        let expected_a_value = 0;
        let expected_state = StateA { a_value: expected_a_value };

        let in_state_a = FlightComputer::<StateA>::new(expected_shared_value_between_states);
        let in_state_d: FlightComputer<StateD> = in_state_a.into();

        */
    }

    // test D can't go to C
    #[test]
    fn test_d_cant_go_to_c() {
        /* The following code does not compile!

        let expected_shared_value_between_states = 10;
        let expected_c_value = 30;
        let expected_state = StateC { c_value: expected_c_value };

        let in_state_a = FlightComputer::<StateA>::new(expected_shared_value_between_states);
        let in_state_c: FlightComputer<StateC> = in_state_a.into();
        let in_state_d: FlightComputer<StateD> = in_state_c.into();
        let in_state_c: FlightComputer<StateC> = in_state_d.into();

        */
    }

    // test B can't go to A
    #[test]
    fn test_b_cant_go_to_a() {
        /* The following code does not compile!

        let expected_shared_value_between_states = 10;
        let expected_a_value = 0;
        let expected_state = StateA { a_value: expected_a_value };

        let in_state_a = FlightComputer::<StateA>::new(expected_shared_value_between_states);
        let in_state_b: FlightComputer<StateB> = in_state_a.into();
        let in_state_a: FlightComputer<StateA> = in_state_b.into();

        */
    }
}
