use common::{comm::{NodeMapping, SamControlMessage, ValveState, VehicleState}, sequence::DeviceAction};
use jeflog::fail;
use pyo3::{types::PyNone, IntoPy, PyObject, Python, ToPyObject};
use std::{net::{ToSocketAddrs, UdpSocket}, sync::Mutex};

use crate::state::SharedState;

pub fn create_device_handler(shared: &SharedState) -> impl Fn(&str, DeviceAction) -> PyObject {
	let vehicle_state = shared.vehicle_state.clone();
	let mappings = shared.mappings.clone();

	let sam_socket = UdpSocket::bind("0.0.0.0:0").unwrap();

	move |device, action| {
		match action {
			DeviceAction::ReadSensor => read_sensor(device, &vehicle_state),
			DeviceAction::ActuateValve { state } => {
				actuate_valve(device, state, &mappings, &sam_socket);
				Python::with_gil(|py| PyNone::get(py).to_object(py))
			},
		}
	}
}

fn read_sensor(name: &str, vehicle_state: &Mutex<VehicleState>) -> PyObject {
	let measurement = vehicle_state
		.lock()
		.unwrap()
		.sensor_readings
		.get(name);

	Python::with_gil(move |py| {
		measurement
			.map_or(
				PyNone::get(py).to_object(py),
				|m| m.into_py(py), 
			)
	})
}

fn actuate_valve(name: &str, state: ValveState, mappings: &Mutex<Vec<NodeMapping>>, sam_socket: &UdpSocket) {
	let mappings = mappings.lock().unwrap();

	let Some(mapping) = mappings.iter().find(|m| m.text_id == name) else {
		fail!("Failed to actuate valve: mapping '{name}' is not defined.");
		return;
	};

	let closed = state == ValveState::Closed;
	let normally_closed = mapping.normally_closed.unwrap_or(true);
	let powered = closed != normally_closed;

	let message = SamControlMessage::ActuateValve { channel: mapping.channel, powered };

	let address = format!("{}.local:8378", mapping.board_id)
		.to_socket_addrs()
		.ok()
		.and_then(|mut addrs| addrs.find(|addr| addr.is_ipv4()));

	if let Some(address) = address {
		let serialized = match postcard::to_allocvec(&message) {
			Ok(serialized) => serialized,
			Err(error) => {
				fail!("Failed to actuate valve: {error}");
				return;
			},
		};

		if let Err(error) = sam_socket.send_to(&serialized, address) {
			fail!("Failed to actuate valve: {error}");
			return;
		}
	} else {
		fail!("Failed to actuate valve: address of board '{}' not found.", mapping.board_id);
	}
}


