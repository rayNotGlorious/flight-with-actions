use pyo3::{pyclass, pymethods};

#[pyclass]
#[derive(Clone, Debug)]
pub struct Valve {
	board_id: u32,
	channel: u32,
	active: bool,
}	

#[pymethods]
impl Valve {
	#[new]
	pub fn new(board_id: u32, channel: u32, active: bool) -> Self {
		Valve { board_id, channel, active }
	}

	pub fn open(&self) {

	}

	pub fn close(&self) {

	}

	pub fn is_open(&self) {

	}

	pub fn is_closed(&self) {

	}
}
