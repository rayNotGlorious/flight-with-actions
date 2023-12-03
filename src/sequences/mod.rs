use pyo3::{Python, PyResult, types::PyModule, pymodule, wrap_pyfunction, Py};
use tracing::debug;

pub mod func;
pub mod sensor;
pub mod unit;
pub mod valve;

#[pymodule]
pub fn libseq(python: Python<'_>, module: &PyModule) -> PyResult<()> {
	module.add_class::<unit::Unit>()?;
	module.add_class::<unit::Duration>()?;
	module.add_class::<unit::Pressure>()?;
	module.add_class::<sensor::Sensor>()?;
	module.add_class::<sensor::PT>()?;
	module.add_class::<valve::Valve>()?;

	module.add("s", Py::new(python, unit::Duration::new(1.0))?)?;
	module.add("ms", Py::new(python, unit::Duration::new(0.001))?)?;
	module.add("us", Py::new(python, unit::Duration::new(0.000001))?)?;
	module.add("psi", Py::new(python, unit::Pressure::new(1.0))?)?;
	module.add("F", Py::new(python, unit::Temperature::new(1.0))?)?;

	let pt1 = sensor::Sensor { name: "pt1".to_string() };
	let pt2 = sensor::Sensor { name: "pt2".to_string() };
	let pt3 = sensor::Sensor { name: "pt3".to_string() };
	let pt4 = sensor::Sensor { name: "pt4".to_string() };
	let pt5 = sensor::Sensor { name: "pt5".to_string() };
	let pt6 = sensor::Sensor { name: "pt6".to_string() };

	let tc1 = sensor::Sensor { name: "tc1".to_string() };
	let tc2 = sensor::Sensor { name: "tc2".to_string() };
	let tc3 = sensor::Sensor { name: "tc3".to_string() };
	let tc4 = sensor::Sensor { name: "tc4".to_string() };
	let tc5 = sensor::Sensor { name: "tc5".to_string() };
	let tc6 = sensor::Sensor { name: "tc6".to_string() };

	module.add("pt1", Py::new(python, pt1)?)?;
	module.add("pt2", Py::new(python, pt2)?)?;
	module.add("pt3", Py::new(python, pt3)?)?;
	module.add("pt4", Py::new(python, pt4)?)?;
	module.add("pt5", Py::new(python, pt5)?)?;
	module.add("pt6", Py::new(python, pt6)?)?;

	module.add("tc1", Py::new(python, tc1)?)?;
	module.add("tc2", Py::new(python, tc2)?)?;
	module.add("tc3", Py::new(python, tc3)?)?;
	module.add("tc4", Py::new(python, tc4)?)?;
	module.add("tc5", Py::new(python, tc5)?)?;
	module.add("tc6", Py::new(python, tc6)?)?;

	module.add("valve1", Py::new(python, valve::Valve::new("valve1".to_string()))?)?;
	module.add("valve2", Py::new(python, valve::Valve::new("valve2".to_string()))?)?;
	module.add("valve3", Py::new(python, valve::Valve::new("valve3".to_string()))?)?;
	module.add("valve4", Py::new(python, valve::Valve::new("valve4".to_string()))?)?;
	module.add("valve5", Py::new(python, valve::Valve::new("valve5".to_string()))?)?;
	module.add("valve6", Py::new(python, valve::Valve::new("valve6".to_string()))?)?;


	module.add_function(wrap_pyfunction!(func::wait_for, module)?)?;
	module.add_function(wrap_pyfunction!(func::wait_until, module)?)?;

	Ok(())
}

pub fn run_python_sequence(string: &String) {
	Python::with_gil(|python| {
		// python.run("from libseq import *", None, None).unwrap();
		let result = PyModule::from_code(python, string, "", "");
		debug!("Sequence completed with result: {:?}", result);
	});
}
