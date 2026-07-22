use pyo3::exceptions::PyTypeError;
#[allow(clippy::wildcard_imports)]
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyList, PyTuple};
use pyo3::IntoPyObjectExt;
use serde::ser::{self, Serialize, SerializeMap, SerializeSeq, Serializer};
use serde_dhall::{NumKind, SimpleValue};
use thiserror::Error;

#[derive(Debug, Error)]
enum DhallPythonError {
    #[error("Conversion error: {error}")]
    InvalidConversion {
        #[from]
        error: serde_dhall::Error,
    },
}

impl From<DhallPythonError> for PyErr {
    fn from(error: DhallPythonError) -> Self {
        match error {
            DhallPythonError::InvalidConversion { error } => {
                PyTypeError::new_err(error.to_string())
            }
        }
    }
}

#[pyfunction]
#[pyo3(signature = (fp, **_kwargs))]
fn load(
    py: Python<'_>,
    fp: &Bound<'_, PyAny>,
    _kwargs: Option<Bound<'_, PyDict>>,
) -> PyResult<Py<PyAny>> {
    let _ = fp.call_method1("seek", (0,));
    let contents = fp.call_method0("read")?;
    loads_impl(py, &contents)
}

#[pyfunction]
#[pyo3(signature = (s, **_kwargs))]
fn loads(
    py: Python<'_>,
    s: &Bound<'_, PyAny>,
    _kwargs: Option<Bound<'_, PyDict>>,
) -> PyResult<Py<PyAny>> {
    loads_impl(py, s)
}

#[pyfunction]
#[pyo3(signature = (obj, sort_keys=None, **_kwargs))]
fn dumps(
    obj: &Bound<'_, PyAny>,
    sort_keys: Option<&Bound<'_, PyAny>>,
    _kwargs: Option<Bound<'_, PyDict>>,
) -> PyResult<String> {
    let sort_keys = match sort_keys {
        Some(value) => value.is_truthy()?,
        None => false,
    };
    let serializable = SerializePyObject { obj, sort_keys };
    serde_dhall::serialize(&serializable)
        .to_string()
        .map_err(|error| DhallPythonError::InvalidConversion { error }.into())
}

#[pyfunction]
#[pyo3(signature = (obj, fp, **_kwargs))]
fn dump(
    obj: &Bound<'_, PyAny>,
    fp: &Bound<'_, PyAny>,
    _kwargs: Option<Bound<'_, PyDict>>,
) -> PyResult<()> {
    let serialized = dumps(obj, None, None)?;
    fp.call_method1("write", (serialized,))?;
    Ok(())
}

#[pymodule]
fn dhall(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(load, m)?)?;
    m.add_function(wrap_pyfunction!(loads, m)?)?;
    m.add_function(wrap_pyfunction!(dump, m)?)?;
    m.add_function(wrap_pyfunction!(dumps, m)?)?;
    Ok(())
}

fn simple_value_to_py(py: Python<'_>, value: &SimpleValue) -> PyResult<Py<PyAny>> {
    match value {
        SimpleValue::Num(num) => match num {
            NumKind::Bool(boolean) => boolean.into_py_any(py),
            NumKind::Natural(natural) => natural.into_py_any(py),
            NumKind::Integer(integer) => integer.into_py_any(py),
            NumKind::Double(double) => f64::from(*double).into_py_any(py),
        },
        SimpleValue::Text(text) => text.into_py_any(py),
        SimpleValue::Optional(inner) => match inner {
            Some(value) => simple_value_to_py(py, value),
            None => Ok(py.None()),
        },
        SimpleValue::List(items) => items
            .iter()
            .map(|item| simple_value_to_py(py, item))
            .collect::<PyResult<Vec<_>>>()?
            .into_py_any(py),
        SimpleValue::Record(fields) => {
            let dict = PyDict::new(py);
            for (key, value) in fields {
                dict.set_item(key, simple_value_to_py(py, value)?)?;
            }
            dict.into_py_any(py)
        }
        SimpleValue::Union(name, payload) => match payload {
            Some(value) => simple_value_to_py(py, value),
            None => name.into_py_any(py),
        },
    }
}

fn loads_impl(py: Python<'_>, s: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let source: String = s.extract().map_err(|error: PyErr| {
        PyTypeError::new_err(format!("the Dhall object must be str: {error:?}"))
    })?;
    let value: SimpleValue = serde_dhall::from_str(&source)
        .parse()
        .map_err(DhallPythonError::from)?;
    simple_value_to_py(py, &value)
}

struct SerializePyObject<'a, 'py> {
    obj: &'a Bound<'py, PyAny>,
    sort_keys: bool,
}

impl Serialize for SerializePyObject<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Ok(dict) = self.obj.cast::<PyDict>() {
            let mut map = serializer.serialize_map(Some(dict.len()))?;
            for (key, value) in dict.iter() {
                if key.is_none() {
                    map.serialize_key("null")?;
                } else if let Ok(boolean) = key.extract::<bool>() {
                    map.serialize_key(if boolean { "true" } else { "false" })?;
                } else if let Ok(string) = key.str() {
                    let text = string
                        .to_str()
                        .map_err(|err| ser::Error::custom(format_args!("{err:?}")))?;
                    map.serialize_key(text)?;
                } else {
                    return Err(ser::Error::custom(format_args!(
                        "Dictionary key is not a string: {key:?}"
                    )));
                }
                map.serialize_value(&SerializePyObject {
                    obj: &value,
                    sort_keys: self.sort_keys,
                })?;
            }
            return map.end();
        }

        if let Ok(list) = self.obj.cast::<PyList>() {
            let mut seq = serializer.serialize_seq(Some(list.len()))?;
            for element in list.iter() {
                seq.serialize_element(&SerializePyObject {
                    obj: &element,
                    sort_keys: self.sort_keys,
                })?;
            }
            return seq.end();
        }

        if let Ok(tuple) = self.obj.cast::<PyTuple>() {
            let mut seq = serializer.serialize_seq(Some(tuple.len()))?;
            for element in tuple.iter() {
                seq.serialize_element(&SerializePyObject {
                    obj: &element,
                    sort_keys: self.sort_keys,
                })?;
            }
            return seq.end();
        }

        if let Ok(string) = self.obj.extract::<String>() {
            return string.serialize(serializer);
        }
        if let Ok(boolean) = self.obj.extract::<bool>() {
            return boolean.serialize(serializer);
        }
        if let Ok(float) = self.obj.cast::<PyFloat>() {
            return float.value().serialize(serializer);
        }
        if let Ok(unsigned) = self.obj.extract::<u64>() {
            return unsigned.serialize(serializer);
        }
        if let Ok(signed) = self.obj.extract::<i64>() {
            return signed.serialize(serializer);
        }
        if self.obj.is_none() {
            return serializer.serialize_unit();
        }

        match self.obj.repr() {
            Ok(repr) => {
                let repr = repr.to_string_lossy();
                Err(ser::Error::custom(format_args!(
                    "Value is not JSON serializable: {repr}"
                )))
            }
            Err(_) => {
                let type_name = self.obj.get_type().name();
                Err(ser::Error::custom(format_args!(
                    "Type is not JSON serializable: {type_name:?}"
                )))
            }
        }
    }
}
