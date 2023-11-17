use std::{sync::Arc, collections::HashMap};

use pyo3::{pyclass, pymethods};

use crate::sequences::unit::Unit;

#[pyclass(subclass)]
#[derive(Clone, Debug)]
pub struct Sensor {
	name: String,
	sensor_map: Arc<HashMap<String, f64>>,
}

#[pymethods]
impl Sensor {
	pub fn read(&self) {

	}
}

#[pyclass(extends = Sensor)]
#[derive(Clone, Debug)]
pub struct PT;
