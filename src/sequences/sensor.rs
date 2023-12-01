use std::sync::RwLock;
use std::{sync::Arc, collections::HashMap};

use pyo3::{pyclass, pymethods};

use crate::sequences::unit::Unit;
use crate::state;

#[pyclass(subclass)]
#[derive(Clone, Debug)]
pub struct Sensor {
	pub name: String,
}

#[pymethods]
impl Sensor {
	pub fn read(&self) -> f64 {
		if let Some(value) = state::read_sensor(&self.name) {
			return value;
		} else {
			return 0.0;
		}
	}
}

#[pyclass(extends = Sensor)]
#[derive(Clone, Debug)]
pub struct PT {
	pub name: String,
}


