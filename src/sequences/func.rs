use crate::unit::Duration;
use pyo3::{PyAny, PyRef, PyResult, pyfunction};
use std::{thread, time::{self, Instant}};

#[pyfunction]
pub fn wait_for(duration: PyRef<'_, Duration>) {
	thread::sleep(time::Duration::from_secs_f64(duration.into_super().raw))
}

#[pyfunction]
pub fn wait_until(condition: &PyAny, interval: Option<PyRef<'_, Duration>>, timeout: Option<PyRef<'_, Duration>>) -> PyResult<()> {
	let interval = interval
		.map_or(time::Duration::from_millis(10), |interval|
			time::Duration::from_secs_f64(interval.into_super().raw)
		);

	let timeout = timeout
		.map_or(time::Duration::MAX, |timeout| {
			time::Duration::from_secs_f64(timeout.into_super().raw)
		});
	
	let end_time = Instant::now() + timeout;

	while !condition.call0()?.is_true()? && Instant::now() < end_time {
		thread::sleep(interval);
	}

	Ok(())
}
