use std::borrow::Cow;
use std::fmt::Write;

use serde::{Deserialize, Serialize};

use crate::Spreadsheet;

use self::cell_content::CellContent;

pub mod cell_content;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct CellPosition(pub usize, pub usize);

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
        let mut result = String::new();
        let letters: Vec<_> = (0u8..26).map(|o| (b'A' + o) as char).collect();
        let mut index = self.position.0;
        while index > 26 {
            result.push(letters[index % 26]);
            index /= 26;
        }
        result.push(letters[index]);
        write!(result, "{}", self.position.1 + 1).unwrap();
        result
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
