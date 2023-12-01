use pyo3::{
	IntoPy,
	PyAny,
	PyObject,
	PyResult,
	basic::CompareOp,
	pyclass,
	pymethods, PyClassInitializer,
};

#[pyclass(subclass)]
#[derive(Clone, Copy, Debug)]
pub struct Unit {
	pub raw: f64
}

#[pymethods]
impl Unit {
	#[new]
	fn new(raw: f64) -> Self {
		Unit { raw }
	}

	fn __add__(&self, other: &Unit) -> Unit {
		Unit { raw: self.raw + other.raw }
	}

	fn __sub__(&self, other: &Unit) -> Unit {
		Unit { raw: self.raw - other.raw }
	}

	fn __mul__(&self, other: &PyAny) -> PyResult<Unit> {
		if let Ok(int) = other.extract::<i64>() {
			Ok(Unit { raw: self.raw * int as f64 })
		} else {
			Ok(Unit { raw: self.raw * other.extract::<f64>()? })
		}
	}

	fn __rmul__(&self, other: &PyAny) -> PyResult<Unit> {
		self.__mul__(other)
	}

	fn __truediv__(&self, other: &PyAny) -> PyResult<PyObject> {
		if let Ok(int) = other.extract::<i64>() {
			Ok(Unit { raw: self.raw / int as f64 }.into_py(other.py()))
		} else if let Ok(float) = other.extract::<f64>() {
			Ok(Unit { raw: self.raw / float }.into_py(other.py()))
		} else {
			Ok((self.raw / other.extract::<Unit>()?.raw).into_py(other.py()))
		}
	}

	fn __iadd__(&mut self, other: &Unit) {
		self.raw += other.raw
	}

	fn __isub__(&mut self, other: &Unit) {
		self.raw -= other.raw
	}

	fn __imul__(&mut self, other: &PyAny) -> PyResult<()> {
		if let Ok(int) = other.extract::<i64>() {
			self.raw *= int as f64;
		} else {
			self.raw *= other.extract::<f64>()?;
		}

		Ok(())
	}

	fn __idiv__(&mut self, other: &PyAny) -> PyResult<()> {
		if let Ok(int) = other.extract::<i64>() {
			self.raw /= int as f64;
		} else {
			self.raw /= other.extract::<f64>()?;
		}

		Ok(())
	}

	fn __richcmp__(&self, other: &Unit, op: CompareOp) -> bool {
		op.matches(self.raw.total_cmp(&other.raw))
	}

	fn __repr__(&self) -> String {
		format!("{} s", self.raw)
	}
}

#[pyclass(extends = Unit)]
#[derive(Clone, Copy, Debug)]
pub struct Duration;

#[pymethods]
impl Duration {
	#[new]
	pub fn new(seconds: f64) -> (Self, Unit) {
		(Duration, Unit { raw: seconds })
	}
}

#[pyclass(extends = Unit)]
#[derive(Clone, Copy, Debug)]
pub struct Pressure;

#[pymethods]
impl Pressure {
	#[new]
	pub fn new(psi: f64) -> (Self, Unit) {
		(Pressure, Unit { raw: psi })
	}
}

#[pyclass(extends = Unit)]
#[derive(Clone, Copy, Debug)]
pub struct Temperature;

#[pymethods]
impl Temperature {
	#[new]
	pub fn new(fahrenheit: f64) -> (Self, Unit) {
		(Temperature, Unit { raw: fahrenheit })
	}
}
