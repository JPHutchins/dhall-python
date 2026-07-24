use pyo3::create_exception;
use pyo3::exceptions::PyException;
#[allow(clippy::wildcard_imports)]
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyList, PyString, PyTuple};
use pyo3::IntoPyObjectExt;
use serde::ser::{self, Serialize, SerializeMap, SerializeSeq, Serializer};
use serde_dhall::{NumKind, SimpleValue};

create_exception!(dhall, DhallError, PyException);

fn dhall_error(py: Python<'_>, message: &str) -> Py<PyAny> {
    DhallError::new_err(message.to_owned())
        .into_value(py)
        .into_any()
}

fn parse(py: Python<'_>, source: &str) -> Py<PyAny> {
    match serde_dhall::from_str(source).parse::<SimpleValue>() {
        Ok(value) => match simple_value_to_py(py, &value) {
            Ok(object) => object,
            Err(error) => dhall_error(py, &error.to_string()),
        },
        Err(error) => dhall_error(py, &error.to_string()),
    }
}

fn serialize_to_string(obj: &Bound<'_, PyAny>, sort_keys: bool) -> Result<String, String> {
    serde_dhall::serialize(&SerializePyObject { obj, sort_keys })
        .to_string()
        .map_err(|error| error.to_string())
}

#[pyfunction]
fn loads(py: Python<'_>, s: &str) -> Py<PyAny> {
    parse(py, s)
}

#[pyfunction]
#[pyo3(signature = (obj, sort_keys=false))]
fn dumps(py: Python<'_>, obj: &Bound<'_, PyAny>, sort_keys: bool) -> Py<PyAny> {
    match serialize_to_string(obj, sort_keys) {
        Ok(text) => PyString::new(py, &text).into_any().unbind(),
        Err(message) => dhall_error(py, &message),
    }
}

#[pyfunction]
fn load(py: Python<'_>, fp: &Bound<'_, PyAny>) -> Py<PyAny> {
    let _ = fp.call_method1("seek", (0,));
    match fp.call_method0("read") {
        Ok(contents) => match contents.extract::<String>() {
            Ok(source) => parse(py, &source),
            Err(error) => dhall_error(py, &error.to_string()),
        },
        Err(error) => dhall_error(py, &error.to_string()),
    }
}

#[pyfunction]
#[pyo3(signature = (obj, fp, sort_keys=false))]
fn dump(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    fp: &Bound<'_, PyAny>,
    sort_keys: bool,
) -> Py<PyAny> {
    match serialize_to_string(obj, sort_keys) {
        Ok(text) => match fp.call_method1("write", (text,)) {
            Ok(_) => py.None(),
            Err(error) => dhall_error(py, &error.to_string()),
        },
        Err(message) => dhall_error(py, &message),
    }
}

#[pymodule]
fn dhall(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("DhallError", m.py().get_type::<DhallError>())?;
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
                    "Value is not serializable to Dhall: {repr}"
                )))
            }
            Err(_) => {
                let type_name = self.obj.get_type().name();
                Err(ser::Error::custom(format_args!(
                    "Type is not serializable to Dhall: {type_name:?}"
                )))
            }
        }
    }
}
