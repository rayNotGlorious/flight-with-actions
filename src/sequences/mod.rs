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

fn main() {
	println!("Hello, world!");

	Python::with_gil(|python| {
		PyModule::from_code
	});
}
