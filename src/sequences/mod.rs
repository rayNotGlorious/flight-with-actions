use pyo3::{Python, PyResult, types::PyModule, pymodule, wrap_pyfunction, Py};

pub mod func;
pub mod sensor;
pub mod unit;
pub mod valve;

#[pymodule]
fn libseq(python: Python<'_>, module: &PyModule) -> PyResult<()> {
	module.add_class::<unit::Unit>()?;
	module.add_class::<unit::Duration>()?;
	module.add_class::<unit::Pressure>()?;

	module.add("s", Py::new(python, unit::Duration::new(1.0))?)?;
	module.add("ms", Py::new(python, unit::Duration::new(0.001))?)?;
	module.add("us", Py::new(python, unit::Duration::new(0.000001))?)?;

	module.add("psi", Py::new(python, unit::Pressure::new(1.0))?)?;

	module.add("F", Py::new(python, unit::Temperature::new(1.0))?)?;

	module.add_function(wrap_pyfunction!(func::wait_for, module)?)?;
	module.add_function(wrap_pyfunction!(func::wait_until, module)?)?;

	Ok(())
}

pub fn run_python_sequence(string: &String) {
	pyo3::append_to_inittab!(libseq);
	// pyo3::append_to_inittab!(libseq::unit);
	Python::with_gil(|python| {
		let result = PyModule::from_code(python, "import libseq; unit = libseq.Unit(5); print(5)", "", "");
		println!("{:?}", result)
	});
}
