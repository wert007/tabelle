use self::cell_content::CellContent;
use crate::{to_column_name, Spreadsheet};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, ops};

pub mod cell_content;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct CellPosition(pub usize, pub usize);

impl CellPosition {
    pub(crate) fn parse(text: &str) -> Result<Self, &str> {
        let mut row = 0;
        let mut column = 0;
        for ch in text.chars() {
            if ch.is_ascii_alphabetic() {
                let digit = (ch.to_ascii_uppercase() as u8 - b'A') as usize;
                column = column * 26 + digit;
            } else if ch.is_ascii_digit() {
                let digit = (ch as u8 - b'0') as usize;
                row = row * 10 + digit;
            } else {
                return Err(text);
            }
        }
        Ok(Self(column, row))
    }

    pub(crate) fn from_index(index: usize, width: usize) -> CellPosition {
        let x = index % width;
        let y = index / width;
        CellPosition(x, y)
    }

    pub fn name(&self) -> String {
        format!("{}{}", crate::to_column_name(self.0), self.1)
    }
}

impl PartialOrd for CellPosition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.1.partial_cmp(&other.1) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for CellPosition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.1.cmp(&other.1).then(self.0.cmp(&other.0))
    }
}

impl ops::Sub for CellPosition {
    type Output = (isize, isize);

    fn sub(self, rhs: Self) -> Self::Output {
        let x = self.0 as isize - rhs.0 as isize;
        let y = self.1 as isize - rhs.1 as isize;
        (x, y)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Cell {
    pub(crate) content: CellContent,
    pub(crate) position: CellPosition,
}

impl Cell {
    pub fn column(&self) -> usize {
        self.position.0
    }

    pub fn row(&self) -> usize {
        self.position.1
    }

    pub fn position(&self) -> (usize, usize) {
        (self.position.0, self.position.1)
    }

    pub fn serialize_display_content(&self) -> Cow<str> {
        self.content.serialize_display()
    }

    pub fn long_display_content(&self) -> Cow<str> {
        self.content.long_display()
    }

    pub fn display_content(&self) -> Cow<str> {
        self.content.display()
    }

    pub fn is_right_aligned(&self) -> bool {
        self.content.is_right_aligned()
    }

    pub fn is_error(&self) -> bool {
        self.content.is_error()
    }

    pub fn evaluate(&mut self, spreadsheet: &Spreadsheet) {
        self.content.evaluate(spreadsheet)
    }

    pub fn name(&self) -> String {
        format!("{}{}", to_column_name(self.position.0), self.position.1)
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

impl std::fmt::Debug for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}) {:?}",
            self.position.0, self.position.1, self.content
        )
    }
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}

impl PartialOrd for Cell {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.position.partial_cmp(&other.position)
    }
}

impl Eq for Cell {}

impl Ord for Cell {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.position.cmp(&other.position)
    }
}
