use std::{
    borrow::Cow,
    fmt::{Display, Write},
};

use pyo3::{
    types::{PyDict, PyFloat, PyList, PyLong, PyString},
    PyAny,
};
use serde::{Deserialize, Serialize};

use crate::{cells::CellPosition, to_column_name, Spreadsheet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Formula {
    pub(super) position: CellPosition,
    pub(super) raw: String,
    pub(super) parsed: String,
    /// Contains the references to other cells in the spreadsheet. The
    /// references are parsed from Formula::raw and are ordered by their occurence
    pub(super) references: Vec<CellReference>,
    pub(super) value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) enum CellReference {
    Cell(CellPosition),
    Row(usize),
    Column(usize),
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
    pub(crate) fn value(&self) -> &Value {
        &self.value
    }

    pub(super) fn push_char(&mut self, ch: char) {
        self.raw.push(ch);
        todo!("Update referenced. Honestly, this code path should probably not be used at all..");
    }

    pub(super) fn is_error(&self) -> bool {
        self.value == Value::Error
    }

    pub(super) fn evaluate(&mut self, spreadsheet: &Spreadsheet) {
        use pyo3::prelude::*;
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            self.value = if self.parsed.is_empty() {
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
                match py.eval(&self.parsed, Some(globals), None) {
                    Ok(it) => it.into(),
                    Err(_) => Value::Error,
                }
            }
        })
    }

    pub(super) fn long_display(&self) -> Cow<str> {
        format!("={}", self.raw).into()
    }

    pub(super) fn display(&self) -> Cow<str> {
        self.value.to_string().into()
    }

    pub(crate) fn is_right_aligned(&self) -> bool {
        matches!(
            self.value,
            Value::Number(_) | Value::FloatNumber(_) | Value::Error
        )
    }

    pub(crate) fn new_at(position: CellPosition) -> Formula {
        Self {
            position,
            raw: String::new(),
            parsed: String::new(),
            references: Vec::new(),
            value: Value::Empty,
        }
    }

    pub(crate) fn moved_to(&self, position: CellPosition, size: (usize, usize)) -> Formula {
        let (x_offset, y_offset) = position - self.position;
        let mut references = self.references.clone();
        let mut raw = self.raw.clone();

        let mut cursor = 0;
        for r in references.iter_mut() {
            let (old, new) = match r {
                CellReference::Cell(c) => {
                    let old = c.name();
                    c.0 = (c.0 as isize + x_offset) as usize;
                    c.1 = (c.1 as isize + y_offset) as usize;
                    let replace_with = c.name();
                    (old, replace_with)
                }
                CellReference::Row(r) => {
                    let old = r.to_string();
                    *r = (*r as isize + y_offset) as usize;
                    let replace_with = r.to_string();
                    (old, replace_with)
                }
                CellReference::Column(c) => {
                    let old = crate::to_column_name(*c);
                    *c = (*c as isize + x_offset) as usize;
                    let replace_with = crate::to_column_name(*c);
                    (old, replace_with)
                }
            };
            cursor += raw[cursor..].find(&old).unwrap();
            raw.replace_range(cursor..cursor + old.len(), &new);
        }
        let (parsed, parsed_referenced) = Self::parse_raw(&raw, size);
        assert_eq!(
            references, parsed_referenced,
            "raw = '{raw}', parsed = '{parsed}', self.raw = {}, self.parsed = {}",
            self.raw, self.parsed
        );

        Formula {
            position,
            parsed,
            raw,
            references,
            value: Value::Empty,
        }
    }

    pub(crate) fn parse_raw(raw: &str, size: (usize, usize)) -> (String, Vec<CellReference>) {
        const SEPERATORS: &str = " ()*-+/,.;[]%!";
        let mut variable_buffer = String::with_capacity(raw.len());
        let mut parsed = String::with_capacity(raw.len());
        let mut references = Vec::new();
        let mut last_position = None;

        for ch in raw.chars() {
            if ch == ':' {
                last_position = crate::cell_name_to_position(&variable_buffer).ok();
                variable_buffer.clear();
            } else if SEPERATORS.contains(ch) {
                if let Some(last_position) = last_position.take() {
                    references.push(CellReference::Cell(CellPosition(
                        last_position.0,
                        last_position.1,
                    )));
                    if let Ok(new_position) = crate::cell_name_to_position(&variable_buffer) {
                        references.push(CellReference::Cell(CellPosition(
                            new_position.0,
                            new_position.1,
                        )));
                        let mut python_code = String::new();
                        for x in last_position.0..=new_position.0 {
                            if !python_code.is_empty() {
                                python_code.push_str(" + ");
                            }
                            write!(
                                python_code,
                                "{}[{}:{}]",
                                crate::to_column_name(x),
                                last_position.1,
                                new_position.1 + 1,
                            )
                            .unwrap();
                        }
                        parsed.push_str(&python_code);
                    } else if let Ok(row) = variable_buffer.parse::<usize>() {
                        references.push(CellReference::Row(row));
                        let python_code = format!(
                            "{}[{}:{}]",
                            crate::to_column_name(last_position.0),
                            last_position.1,
                            row + 1,
                        );
                        parsed.push_str(&python_code);
                    } else {
                        dbg!(ch, last_position, parsed, variable_buffer);
                        todo!("This should probaly not happen. This would mean you had a valid first cell, but after the column your cell becomes invalid...");
                    }
                } else {
                    if let Ok(cell) = crate::cell_name_to_position(&variable_buffer) {
                        references.push(CellReference::Cell(CellPosition(cell.0, cell.1)));
                    } else if let Ok(column) = crate::column_name_to_index(&variable_buffer) {
                        if column < size.0 {
                            references.push(CellReference::Column(column));
                        }
                    }
                    parsed.push_str(&variable_buffer);
                }
                variable_buffer.clear();
                if !parsed.is_empty() || !ch.is_whitespace() {
                    parsed.push(ch);
                }
            } else {
                variable_buffer.push(ch);
            }
        }
        if let Some(last_position) = last_position.take() {
            references.push(CellReference::Cell(CellPosition(
                last_position.0,
                last_position.1,
            )));
            if let Ok(new_position) = crate::cell_name_to_position(&variable_buffer) {
                references.push(CellReference::Cell(CellPosition(
                    new_position.0,
                    new_position.1,
                )));
                let mut python_code = String::new();
                for x in last_position.0..=new_position.0 {
                    if !python_code.is_empty() {
                        python_code.push_str(" + ");
                    }
                    write!(
                        python_code,
                        "{}[{}:{}]",
                        crate::to_column_name(x),
                        last_position.1,
                        new_position.1 + 1,
                    )
                    .unwrap();
                }
                parsed.push_str(&python_code);
            } else if let Ok(row) = variable_buffer.parse::<usize>() {
                references.push(CellReference::Row(row));
                let python_code = format!(
                    "{}[{}:{}]",
                    crate::to_column_name(last_position.0),
                    last_position.1,
                    row + 1,
                );
                parsed.push_str(&python_code);
            } else {
                dbg!(last_position, parsed, variable_buffer);
                todo!("This should probaly not happen. This would mean you had a valid first cell, but after the column your cell becomes invalid...");
            }
        } else {
            if let Ok(cell) = crate::cell_name_to_position(&variable_buffer) {
                references.push(CellReference::Cell(CellPosition(cell.0, cell.1)));
            } else if let Ok(column) = crate::column_name_to_index(&variable_buffer) {
                if column < size.0 {
                    references.push(CellReference::Column(column));
                }
            }
            parsed.push_str(&variable_buffer);
        }
        variable_buffer.clear();

        (parsed, references)
    }
}
