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

#[cfg(test)]
mod tests {
    use super::{dhall_error, dump, dumps, load, loads, DhallError};
    use pyo3::ffi::c_str;
    use pyo3::types::{PyAnyMethods, PyStringMethods, PyTypeMethods};
    use pyo3::{Bound, Py, PyAny, Python};
    use std::ffi::CStr;

    fn is_dhall_error(py: Python<'_>, value: &Py<PyAny>) -> bool {
        value
            .bind(py)
            .is_instance(&py.get_type::<DhallError>())
            .unwrap()
    }

    fn eval<'py>(py: Python<'py>, expr: &CStr) -> Bound<'py, PyAny> {
        py.eval(expr, None, None).unwrap()
    }

    fn dumped(py: Python<'_>, expr: &CStr, sort_keys: bool) -> Py<PyAny> {
        dumps(py, &eval(py, expr), sort_keys)
    }

    fn dumped_str(py: Python<'_>, expr: &CStr, sort_keys: bool) -> String {
        dumped(py, expr, sort_keys).extract::<String>(py).unwrap()
    }

    #[test]
    fn parse_bool() {
        Python::attach(|py| {
            assert!(loads(py, "True").extract::<bool>(py).unwrap());
            assert!(!loads(py, "False").extract::<bool>(py).unwrap());
        });
    }

    #[test]
    fn parse_natural() {
        Python::attach(|py| {
            assert_eq!(loads(py, "0").extract::<u64>(py).unwrap(), 0);
            assert_eq!(loads(py, "42").extract::<u64>(py).unwrap(), 42);
        });
    }

    #[test]
    fn parse_integer() {
        Python::attach(|py| {
            assert_eq!(loads(py, "-5").extract::<i64>(py).unwrap(), -5);
            assert_eq!(loads(py, "+7").extract::<i64>(py).unwrap(), 7);
        });
    }

    #[test]
    fn parse_double() {
        Python::attach(|py| {
            let value = loads(py, "3.5").extract::<f64>(py).unwrap();
            assert!((value - 3.5).abs() < f64::EPSILON);
        });
    }

    #[test]
    fn parse_text() {
        Python::attach(|py| {
            assert_eq!(
                loads(py, "\"hello\"").extract::<String>(py).unwrap(),
                "hello"
            );
            assert_eq!(
                loads(py, "\"h\u{e9}llo \u{2603}\"")
                    .extract::<String>(py)
                    .unwrap(),
                "h\u{e9}llo \u{2603}"
            );
        });
    }

    #[test]
    fn parse_optional_some() {
        Python::attach(|py| {
            assert_eq!(loads(py, "Some 1").extract::<u64>(py).unwrap(), 1);
        });
    }

    #[test]
    fn parse_optional_none() {
        Python::attach(|py| {
            assert!(loads(py, "None Natural").is_none(py));
        });
    }

    #[test]
    fn parse_list() {
        Python::attach(|py| {
            assert_eq!(
                loads(py, "[1, 2, 3]").extract::<Vec<u64>>(py).unwrap(),
                vec![1, 2, 3]
            );
        });
    }

    #[test]
    fn parse_empty_list() {
        Python::attach(|py| {
            assert!(loads(py, "[] : List Natural")
                .extract::<Vec<u64>>(py)
                .unwrap()
                .is_empty());
        });
    }

    #[test]
    fn parse_record() {
        Python::attach(|py| {
            let value = loads(py, "{ a = 1, b = \"x\" }");
            let record = value.bind(py);
            assert_eq!(record.get_item("a").unwrap().extract::<u64>().unwrap(), 1);
            assert_eq!(
                record.get_item("b").unwrap().extract::<String>().unwrap(),
                "x"
            );
        });
    }

    #[test]
    fn parse_empty_record() {
        Python::attach(|py| {
            assert_eq!(loads(py, "{=}").bind(py).len().unwrap(), 0);
        });
    }

    #[test]
    fn parse_union_without_payload() {
        Python::attach(|py| {
            assert_eq!(loads(py, "< A | B >.A").extract::<String>(py).unwrap(), "A");
        });
    }

    #[test]
    fn parse_union_with_payload() {
        Python::attach(|py| {
            assert_eq!(
                loads(py, "< A : Natural | B >.A 1")
                    .extract::<u64>(py)
                    .unwrap(),
                1
            );
        });
    }

    #[test]
    fn parse_nested() {
        Python::attach(|py| {
            let value = loads(py, "{ xs = [ { y = 1 }, { y = 2 } ] }");
            let xs = value.bind(py).get_item("xs").unwrap();
            assert_eq!(
                xs.get_item(0)
                    .unwrap()
                    .get_item("y")
                    .unwrap()
                    .extract::<u64>()
                    .unwrap(),
                1
            );
        });
    }

    #[test]
    fn parse_error_syntax() {
        Python::attach(|py| assert!(is_dhall_error(py, &loads(py, "{ a = }"))));
    }

    #[test]
    fn parse_error_type_mismatch() {
        Python::attach(|py| assert!(is_dhall_error(py, &loads(py, "1 + True"))));
    }

    #[test]
    fn parse_error_not_a_simple_value() {
        Python::attach(|py| assert!(is_dhall_error(py, &loads(py, "\\(x : Natural) -> x"))));
    }

    #[test]
    fn dumps_record_single_key() {
        Python::attach(|py| assert_eq!(dumped_str(py, c_str!("{'a': 1}"), false), "{ a = 1 }"));
    }

    #[test]
    fn dumps_record_matches_reference() {
        Python::attach(|py| {
            assert_eq!(
                dumped_str(
                    py,
                    c_str!("{'keyA': 81, 'keyB': True, 'keyC': 'value'}"),
                    false
                ),
                "{ keyA = 81, keyB = True, keyC = \"value\" }"
            );
        });
    }

    #[test]
    fn dumps_list() {
        Python::attach(|py| assert_eq!(dumped_str(py, c_str!("[1, 2, 3]"), false), "[1, 2, 3]"));
    }

    #[test]
    fn dumps_tuple_as_list() {
        Python::attach(|py| {
            assert_eq!(dumped_str(py, c_str!("(1, 2, 3)"), false), "[1, 2, 3]");
        });
    }

    #[test]
    fn dumps_text() {
        Python::attach(|py| assert_eq!(dumped_str(py, c_str!("'hello'"), false), "\"hello\""));
    }

    #[test]
    fn dumps_bool() {
        Python::attach(|py| {
            assert_eq!(dumped_str(py, c_str!("True"), false), "True");
            assert_eq!(dumped_str(py, c_str!("False"), false), "False");
        });
    }

    #[test]
    fn dumps_natural() {
        Python::attach(|py| assert_eq!(dumped_str(py, c_str!("7"), false), "7"));
    }

    #[test]
    fn dumps_negative_integer() {
        Python::attach(|py| assert_eq!(dumped_str(py, c_str!("-5"), false), "-5"));
    }

    #[test]
    fn dumps_double() {
        Python::attach(|py| assert_eq!(dumped_str(py, c_str!("3.5"), false), "3.5"));
    }

    #[test]
    fn dumps_none_is_empty_record() {
        Python::attach(|py| assert_eq!(dumped_str(py, c_str!("None"), false), "{=}"));
    }

    #[test]
    fn sort_keys_matches_default() {
        Python::attach(|py| {
            assert_eq!(
                dumped_str(py, c_str!("{'b': 1, 'a': 2}"), true),
                dumped_str(py, c_str!("{'b': 1, 'a': 2}"), false)
            );
        });
    }

    #[test]
    fn dumps_error_set() {
        Python::attach(|py| assert!(is_dhall_error(py, &dumped(py, c_str!("{1, 2, 3}"), false))));
    }

    #[test]
    fn dumps_error_complex() {
        Python::attach(|py| assert!(is_dhall_error(py, &dumped(py, c_str!("1j"), false))));
    }

    #[test]
    fn round_trip_record() {
        Python::attach(|py| {
            let obj = eval(py, c_str!("{'a': 1, 'b': [1, 2], 'c': 'x'}"));
            let text = dumps(py, &obj, false).extract::<String>(py).unwrap();
            assert!(loads(py, &text).bind(py).eq(&obj).unwrap());
        });
    }

    #[test]
    fn dhall_error_is_instance_with_message() {
        Python::attach(|py| {
            let error = dhall_error(py, "boom");
            assert!(is_dhall_error(py, &error));
            assert_eq!(
                error.bind(py).str().unwrap().to_str().unwrap().to_owned(),
                "boom"
            );
        });
    }

    #[test]
    fn load_reads_from_file_like() {
        Python::attach(|py| {
            let fp = py
                .import("io")
                .unwrap()
                .call_method1("StringIO", ("{ a = 1 }",))
                .unwrap();
            let value = load(py, &fp);
            assert_eq!(
                value
                    .bind(py)
                    .get_item("a")
                    .unwrap()
                    .extract::<u64>()
                    .unwrap(),
                1
            );
        });
    }

    #[test]
    fn dump_writes_to_file_like() {
        Python::attach(|py| {
            let fp = py.import("io").unwrap().call_method0("StringIO").unwrap();
            let obj = eval(py, c_str!("{'a': 1}"));
            assert!(dump(py, &obj, &fp, false).is_none(py));
            assert_eq!(
                fp.call_method0("getvalue")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "{ a = 1 }"
            );
        });
    }

    #[test]
    fn dump_reports_serialization_error() {
        Python::attach(|py| {
            let fp = py.import("io").unwrap().call_method0("StringIO").unwrap();
            let obj = eval(py, c_str!("1j"));
            assert!(is_dhall_error(py, &dump(py, &obj, &fp, false)));
        });
    }

    #[test]
    fn type_name_of_dhall_error() {
        Python::attach(|py| {
            assert_eq!(py.get_type::<DhallError>().name().unwrap(), "DhallError");
        });
    }
}
