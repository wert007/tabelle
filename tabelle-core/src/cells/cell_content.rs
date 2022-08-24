use std::borrow::Cow;

use pyo3::{Py, PyAny, Python, ToPyObject};
use serde::{Deserialize, Serialize};

use crate::Spreadsheet;

use self::formula::{Formula, Value};

use super::CellPosition;

mod formula;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum CellContent {
    #[default]
    Empty,
    Text(String),
    Number(i64),
    FloatNumber(f64, i32),
    Formula(Formula),
}

impl CellContent {
    pub(super) fn is_right_aligned(&self) -> bool {
        match self {
            CellContent::Empty => false,
            CellContent::Text(_) => false,
            CellContent::Number(_) => true,
            CellContent::FloatNumber(..) => true,
            CellContent::Formula(f) => f.is_right_aligned(),
        }
    }

    pub(super) fn evaluate(&mut self, spreadsheet: &Spreadsheet) {
        if let CellContent::Formula(it) = self {
            it.evaluate(spreadsheet)
        }
    }

    pub fn serialize_display(&self) -> Cow<str> {
        if let CellContent::Formula(it) = self {
            it.long_display()
        } else {
            self.display()
        }
    }

    pub fn long_display(&self) -> Cow<str> {
        match self {
            CellContent::Empty => "Insert text..".into(),
            CellContent::Formula(it) => it.long_display(),
            _ => self.display(),
        }
    }

    pub(super) fn display(&self) -> Cow<str> {
        match self {
            CellContent::Empty => "".into(),
            CellContent::Text(it) => it.into(),
            CellContent::Number(it) => it.to_string().into(),
            CellContent::FloatNumber(it, _) => it.to_string().into(),
            CellContent::Formula(it) => it.display(),
        }
    }

    pub(crate) fn input_char(&mut self, ch: char, position: CellPosition) {
        match self {
            empty @ CellContent::Empty => {
                *empty = match ch.to_digit(10) {
                    Some(digit) => CellContent::Number(digit as i64),
                    None => match ch {
                        '=' => CellContent::Formula(Formula::new_at(position)),
                        '.' => CellContent::FloatNumber(0.0, 1),
                        ch => CellContent::Text(ch.to_string()),
                    },
                };
            }
            CellContent::Text(it) => it.push(ch),
            CellContent::Formula(f) => {
                f.push_char(ch);
            }
            CellContent::Number(it) if ch.is_ascii_digit() => {
                let digit = ch.to_digit(10).unwrap();
                *it *= 10;
                *it += digit as i64;
            }
            cell @ CellContent::Number(_) if ch == '.' => {
                *cell = CellContent::FloatNumber(cell.as_number().unwrap() as f64, 1);
            }
            cell @ CellContent::Number(_) => {
                *cell = CellContent::Text(format!("{}{ch}", cell.as_number().unwrap()));
            }
            CellContent::FloatNumber(it, digit_count) if ch.is_ascii_digit() => {
                let digit = ch.to_digit(10).unwrap() as f64;
                let digit = digit / 10.0f64.powi(*digit_count);
                *digit_count += 1;
                *it += digit;
            }
            cell @ CellContent::FloatNumber(..) => {
                *cell = CellContent::Text(format!("{}{ch}", cell.as_float_number().unwrap()));
            }
        }
    }

    pub fn as_number(&self) -> Option<i64> {
        if let Self::Number(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn as_float_number(&self) -> Option<f64> {
        if let Self::FloatNumber(v, _) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Returns `true` if the cell content is [`Error`].
    ///
    /// [`Error`]: CellContent::Error
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Formula(f) if f.is_error())
    }

    /// Returns `true` if the cell content is [`Empty`].
    ///
    /// [`Empty`]: CellContent::Empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub(crate) fn parse(cell: &str, cell_position: CellPosition) -> CellContent {
        if cell.is_empty() {
            CellContent::Empty
        } else {
            match cell.parse::<i64>() {
                Ok(it) => CellContent::Number(it),
                Err(_) => match cell.parse::<f64>() {
                    Ok(it) => CellContent::FloatNumber(it, 0),
                    Err(_) => {
                        if cell.chars().next().unwrap() == '=' {
                            CellContent::Formula(Formula {
                                position: cell_position,
                                buffer: cell[1..].to_owned(),
                                value: Value::Empty,
                            })
                        } else {
                            CellContent::Text(cell.into())
                        }
                    }
                },
            }
        }
    }

    pub(crate) fn try_to_object(&self, py: Python) -> Option<Py<PyAny>> {
        match &self {
            CellContent::Empty => None,
            CellContent::Text(it) => Some(it.to_object(py)),
            CellContent::Number(it) => Some(it.to_object(py)),
            CellContent::FloatNumber(it, _) => Some(it.to_object(py)),
            CellContent::Formula(it) => match &it.value {
                Value::String(it) => Some(it.to_object(py)),
                Value::Number(it) => Some(it.to_object(py)),
                Value::FloatNumber(it) => Some(it.to_object(py)),
                _ => None,
            },
        }
    }
}
