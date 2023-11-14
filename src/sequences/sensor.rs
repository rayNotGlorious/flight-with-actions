use std::{sync::Arc, collections::HashMap};

use pyo3::{pyclass, pymethods};

use crate::unit::Unit;

#[pyclass(subclass)]
#[derive(Clone, Debug)]
pub struct Sensor {
	name: String,
	sensor_map: Arc<HashMap<String, f64>>,
}

#[pymethods]
impl Sensor {
	#[new]
	pub fn new(name: String, sensor_map: Arc<HashMap<String, f64>>) -> Self {
		Sensor { name, sensor_map }
	}

	pub fn read(&self) -> Unit {

	}
}

#[pyclass(extends = Sensor)]
#[derive(Clone, Debug)]
pub struct PT;
