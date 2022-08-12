use std::{borrow::Cow, fmt::Display};

use pyo3::{
    types::{PyDict, PyFloat, PyList, PyLong, PyString},
    PyAny,
};
use serde::{Deserialize, Serialize};

use crate::{cells::CellPosition, to_column_name, Spreadsheet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula {
    position: CellPosition,
    buffer: String,
    pub(super) value: Value,
}

#[derive(Debug, PartialEq, Default, Clone, Serialize, Deserialize)]
pub(crate) enum Value {
    String(String),
    Number(i64),
    FloatNumber(f64),
    #[default]
    Empty,
    Error,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(it) => write!(f, "{it}"),
            Value::Number(it) => write!(f, "{it}"),
            Value::FloatNumber(it) => write!(f, "{it}"),
            Value::Empty => write!(f, ""),
            Value::Error => write!(f, "#error"),
        }
    }
}

impl From<&PyAny> for Value {
    fn from(it: &PyAny) -> Self {
        match it.downcast::<PyFloat>() {
            Ok(it) => Value::FloatNumber(it.value()),
            Err(_) => match it.downcast::<PyLong>() {
                Ok(it) => match it.extract::<i64>() {
                    Ok(it) => Value::Number(it),
                    Err(_) => Value::Error,
                },
                Err(_) => match it.downcast::<PyString>() {
                    Ok(it) => Value::String(it.to_string()),
                    Err(_) => Value::Error,
                },
            },
        }
    }
}

impl Formula {
    pub(super) fn push_char(&mut self, ch: char) {
        self.buffer.push(ch)
    }

    pub(super) fn is_error(&self) -> bool {
        self.value == Value::Error
    }

    pub(super) fn evaluate(&mut self, spreadsheet: &Spreadsheet) {
        use pyo3::prelude::*;
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            self.value = if self.buffer.is_empty() {
                Value::Empty
            } else {
                let globals = PyDict::new(py);
                let modules = ["random", "math"];
                for module in modules {
                    let py_module = py.import(module).unwrap();
                    globals.set_item(module.to_object(py), py_module).unwrap();
                }
                for cell in &spreadsheet.cells {
                    if cell.position == self.position {
                        continue;
                    }
                    let names = [cell.name(), cell.name().to_lowercase()];
                    for name in names {
                        let name = PyString::new(py, &name);
                        if let Some(value) = cell.content.try_to_object(py) {
                            let _ = globals.set_item(name, value);
                        }
                    }
                }
                for i in 0..spreadsheet.columns() {
                    let name = to_column_name(i);
                    let names = [name.clone(), name.to_lowercase()];
                    for name in names {
                        let name = name.to_object(py);
                        let list = PyList::empty(py);
                        for cell in spreadsheet.into_iter().filter(|c| c.position.0 == i) {
                            if cell.position == self.position {
                                continue;
                            }
                            if let Some(value) = cell.content.try_to_object(py) {
                                let _ = list.append(value);
                            }
                        }
                        let _ = globals.set_item(name, list);
                    }
                }
                match py.eval(self.buffer.trim(), Some(globals), None) {
                    Ok(it) => it.into(),
                    Err(_) => Value::Error,
                }
            }
        })
    }

    pub(super) fn long_display(&self) -> Cow<str> {
        format!("= {}", self.buffer).into()
    }

    pub(super) fn display(&self) -> Cow<str> {
        self.value.to_string().into()
    }

    pub(crate) fn is_right_aligned(&self) -> bool {
        match &self.value {
            Value::Number(_) => true,
            Value::FloatNumber(_) => true,
            Value::Error => true,
            _ => false,
        }
    }

    pub(crate) fn new_at(position: CellPosition) -> Formula {
        Self {
            position,
            buffer: String::new(),
            value: Value::Empty,
        }
    }
}
