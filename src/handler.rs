use common::{comm::{BoardId, CompositeValveState, NodeMapping, SamControlMessage, ValveState, VehicleState}, sequence::{AbortError, DeviceAction}};
use jeflog::{task, fail, pass};
use pyo3::{types::PyNone, IntoPy, PyErr, PyObject, Python, ToPyObject};
use std::{sync::{mpsc::Sender, Mutex}, thread};

use crate::state::SharedState;

pub fn create_device_handler(shared: &SharedState, command_tx: Sender<(BoardId, SamControlMessage)>) -> impl Fn(&str, DeviceAction) -> PyObject {
	let vehicle_state = shared.vehicle_state.clone();
	let sequences = shared.sequences.clone();
	let mappings = shared.mappings.clone();
	let tx = command_tx.clone();

	move |device, action| {
		let thread_id = thread::current().id();
		let sequences = sequences.lock().unwrap();
		
		if sequences.get_by_right(&thread_id).is_none() {
			Python::with_gil(|py| {
				AbortError::new_err("aborting sequence").restore(py);
				assert!(PyErr::occurred(py));
				drop(PyErr::fetch(py));
			})
		}

		match action {
			DeviceAction::ReadSensor => read_sensor(device, &vehicle_state),
			DeviceAction::ReadValveState => read_valve_state(device, &vehicle_state),
			DeviceAction::ActuateValve { state } => {
				actuate_valve(device, state, &mappings, &vehicle_state, &tx);
				Python::with_gil(|py| PyNone::get(py).to_object(py))
			},
		}
	}
}

fn read_sensor(name: &str, vehicle_state: &Mutex<VehicleState>) -> PyObject {
	let vehicle_state = vehicle_state
		.lock()
		.unwrap();

	let measurement = vehicle_state
		.sensor_readings
		.get(name);

	Python::with_gil(move |py| {
		measurement
			.map_or(
				PyNone::get(py).to_object(py),
				|m| m.clone().into_py(py), 
			)
	})
}

fn read_valve_state(name: &str, vehicle_state: &Mutex<VehicleState>) -> PyObject {
	let vehicle_state = vehicle_state
		.lock()
		.unwrap();

	let state = vehicle_state
		.valve_states
		.get(name);

	Python::with_gil(|py| {
		state.map_or(
			PyNone::get(py).to_object(py),
			|s| s.actual.to_string().into_py(py),
		)
	})
}

fn actuate_valve(name: &str, state: ValveState, mappings: &Mutex<Vec<NodeMapping>>, vehicle_state: &Mutex<VehicleState>, command_tx: &Sender<(BoardId, SamControlMessage)>) {
	let mappings = mappings.lock().unwrap();

	let Some(mapping) = mappings.iter().find(|m| m.text_id == name) else {
		fail!("Failed to actuate valve: mapping '{name}' is not defined.");
		return;
	};

	let closed = state == ValveState::Closed;
	let normally_closed = mapping.normally_closed.unwrap_or(true);
	let powered = closed != normally_closed;

	let message = SamControlMessage::ActuateValve { channel: mapping.channel, powered };

	task!("Sending SamControlMessage::ActuateValve to {}", mapping.board_id);
	match command_tx.send((mapping.board_id.clone(), message)) {
		Ok(()) => pass!("Command sent!"),
		Err(e) => fail!("Command couldn't be sent: {e}")
	}

	let mut vehicle_state = vehicle_state.lock().unwrap();

	if let Some(existing) = vehicle_state.valve_states.get_mut(name) {
		existing.commanded = state;
	} else {
		vehicle_state.valve_states.insert(name.to_owned(), CompositeValveState {
			commanded: state,
			actual: ValveState::Undetermined
		});
	}
}


