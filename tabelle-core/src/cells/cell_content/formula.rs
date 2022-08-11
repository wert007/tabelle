use std::{borrow::Cow, fmt::Display};

use pyo3::{
    types::{PyDict, PyFloat, PyLong, PyString},
    PyAny,
};

use crate::{cells::cell_content::CellContent, Spreadsheet};

#[derive(Debug, Default, Clone)]
pub struct Formula {
    buffer: String,
    value: Value,
}

#[derive(Debug, PartialEq, Default, Clone)]
enum Value {
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
                for cell in &spreadsheet.cells {
                    let name = cell.name();
                    let name = PyString::new(py, &name);
                    let _ = match &cell.content {
                        CellContent::Empty => Ok(()),
                        CellContent::Text(it) => globals.set_item(name, PyString::new(py, it)),
                        CellContent::Number(it) => globals.set_item(name, it.to_object(py)),
                        CellContent::FloatNumber(it, _) => globals.set_item(name, it.to_object(py)),
                        CellContent::Formula(it) => match &it.value {
                            Value::String(it) => globals.set_item(name, PyString::new(py, it)),
                            Value::Number(it) => globals.set_item(name, it.to_object(py)),
                            _ => Ok(()),
                        },
                    };
                }
                match py.eval(&self.buffer, Some(globals), None) {
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
}
