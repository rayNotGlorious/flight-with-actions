use crate::sequences::unit::Unit;
use pyo3::{PyAny, PyRef, PyResult, pyfunction};
use std::{thread, time::{self, Instant}};

#[pyfunction]
pub fn wait_for(unit: PyRef<'_, Unit>) {
	thread::sleep(time::Duration::from_secs_f64(unit.raw))
}

#[pyfunction]
pub fn wait_until(condition: &PyAny, interval: Option<PyRef<'_, Unit>>, timeout: Option<PyRef<'_, Unit>>) -> PyResult<()> {
	let interval = interval
		.map_or(time::Duration::from_millis(10), |interval|
			time::Duration::from_secs_f64(interval.raw)
		);

	let timeout = timeout
		.map_or(time::Duration::MAX, |timeout| {
			time::Duration::from_secs_f64(timeout.raw)
		});
	
	let end_time = Instant::now() + timeout;

	while !condition.call0()?.is_true()? && Instant::now() < end_time {
		thread::sleep(interval);
	}

	Ok(())
}
