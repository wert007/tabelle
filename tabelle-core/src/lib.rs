use cells::{Cell, CellPosition};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Write,
    path::{Path, PathBuf},
};
use unicode_width::UnicodeWidthStr;
use units::UnitKind;
mod cells;
pub mod units;
pub use cells::cell_content::CellContent;

pub fn dump(path: &str) {
    _ = dbg!(umya_spreadsheet::reader::xlsx::read(path));
    panic!("damn");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spreadsheet {
    current_cell: CellPosition,
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    column_widths: Vec<usize>,
    used_cells: CellPosition,
    fixed_rows: usize,
    path: Option<PathBuf>,
}

impl Spreadsheet {
    pub fn new(width: usize, height: usize) -> Self {
        let mut cells = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                cells.push(Cell {
                    content: CellContent::default(),
                    position: CellPosition(x, y),
                    unit: UnitKind::None,
                });
            }
        }
        let column_widths = std::iter::repeat(10).take(width).collect();
        Self {
            current_cell: CellPosition(0, 0),
            width,
            height,
            cells,
            used_cells: CellPosition(0, 0),
            column_widths,
            fixed_rows: 0,
            path: None,
        }
    }

    pub fn load_csv(csv: &str) -> Self {
        let mut width = 0;
        let mut cells = vec![];
        let mut column = 0;
        let mut row = 0;
        let mut needs_evaluation = false;
        for line in csv.lines() {
            if line.is_empty() {
                continue;
            }
            if line.chars().all(|c| c.is_whitespace()) {
                continue;
            }
            for cell in line.split(',') {
                let cell = cell.trim();
                if cell.starts_with('=') {
                    needs_evaluation = true;
                }
                cells.push(Cell {
                    content: CellContent::parse(cell, (column, row), (usize::MAX, usize::MAX)),
                    position: CellPosition(column, row),
                    unit: UnitKind::None,
                });
                column += 1;
            }
            width = width.max(column);
            column = 0;
            row += 1;
        }
        let height = row;
        let column_widths = std::iter::repeat(10).take(width).collect();
        let mut result = Self {
            current_cell: CellPosition(0, 0),
            width,
            height,
            cells,
            used_cells: CellPosition(0, 0),
            column_widths,
            fixed_rows: 0,
            path: None,
        };
        // This is very brute forcey. Could be fixed probably.
        if needs_evaluation {
            for _ in 0..width * height {
                result.evaluate()
            }
        }
        assert_eq!(width * height, result.cells.len());
        result
    }

    pub fn load_xlsx(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let spreadsheet = umya_spreadsheet::reader::xlsx::read(path).unwrap();
        let worksheet = spreadsheet.get_sheet(&0).unwrap();
        let (width, height) = worksheet.get_highest_column_and_row();
        let (width, height) = (width as usize, height as usize);
        let current_cell = CellPosition::parse(worksheet.get_active_cell()).unwrap();
        let mut cells = Vec::with_capacity(width * height);
        let mut column_widths = vec![10; width];
        let mut needs_evaluation = false;
        for y in 0..height {
            for (x, column_width) in column_widths.iter_mut().enumerate() {
                let col = (x as u32) + 1;
                let row = (y as u32) + 1;
                *column_width = worksheet
                    .get_column_dimension_by_number(&col)
                    .map(|c| (*c.get_width()) as usize)
                    .unwrap_or(10);
                let unit = worksheet
                    .get_style_by_column_and_row(&col, &row)
                    .get_numbering_format()
                    .as_ref()
                    .and_then(|n| UnitKind::try_from(n).ok())
                    .unwrap_or_default();
                let content = if let Some(cell) = worksheet.get_cell_by_column_and_row(&col, &row) {
                    if cell.is_formula() || cell.get_value().starts_with('=') {
                        needs_evaluation = true;
                    }
                    CellContent::parse(&cell.get_value(), (x, y), (width, height))
                } else {
                    CellContent::Empty
                };
                cells.push(Cell {
                    content,
                    position: CellPosition(x, y),
                    unit,
                })
            }
        }
        assert_eq!(cells.len(), width * height);
        let mut result = Self {
            current_cell,
            width,
            height,
            cells,
            used_cells: CellPosition(width, height),
            column_widths,
            fixed_rows: 0,
            path: Some(path.into()),
        };
        // This is very brute forcey. Could be fixed probably.
        if needs_evaluation {
            for _ in 0..width * height {
                result.evaluate()
            }
        }
        result
    }

    pub fn columns(&self) -> usize {
        self.width
    }

    pub fn rows(&self) -> usize {
        self.height
    }

    pub fn column_width(&self, column: usize) -> usize {
        self.column_widths[column]
    }

    pub fn set_column_width(&mut self, column: usize, width: usize) {
        self.column_widths[column] = width;
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        let additional = width * height - self.cells.len();
        self.column_widths.reserve(width - self.column_widths.len());
        self.cells.reserve(additional);
        for x in 0..width {
            for y in self.height..height {
                self.cells.push(Cell {
                    content: CellContent::Empty,
                    position: CellPosition(x, y),
                    unit: UnitKind::None,
                });
            }
        }
        for x in self.width..width {
            self.column_widths.push(10);
            for y in 0..self.height {
                self.cells.push(Cell {
                    content: CellContent::Empty,
                    position: CellPosition(x, y),
                    unit: UnitKind::None,
                });
            }
        }
        self.cells.sort();
        self.width = width;
        self.height = height;
    }

    pub fn find(&self, text: &str) -> Option<(usize, usize)> {
        let index = self.index(self.current_cell());
        let mut cells = self.cells[index..]
            .iter()
            .skip(1)
            .chain(self.cells[..index].iter());
        cells.find_map(|c| match &c.content {
            CellContent::Empty => None,
            CellContent::Text(it) => {
                if it.contains(text) {
                    Some(c.position())
                } else {
                    None
                }
            }
            CellContent::Number(_) => None,
            CellContent::FloatNumber(_, _) => None,
            CellContent::Formula(_) => None,
        })
    }

    pub fn set_cursor(&mut self, cell_position: (usize, usize)) {
        self.current_cell = CellPosition(cell_position.0, cell_position.1);
    }

    pub fn move_cursor(&mut self, x: isize, y: isize) -> bool {
        let mut result = true;
        let x = self.current_cell.0 as isize + x;
        let y = self.current_cell.1 as isize + y;
        let x = if x < 0 {
            result = false;
            0
        } else if x as usize >= self.width {
            result = false;
            self.width - 1
        } else {
            x as usize
        };
        let y = if y < 0 {
            result = false;
            0
        } else if y as usize >= self.height {
            result = false;
            self.height - 1
        } else {
            y as usize
        };
        self.current_cell = CellPosition(x, y);
        result
    }

    pub fn input_char(&mut self, ch: char) {
        self.used_cells = CellPosition(
            self.current_cell.0.max(self.used_cells.0),
            self.current_cell.1.max(self.used_cells.1),
        );
        let index = self.index(self.current_cell());
        self.cells[index].content.input_char(ch, self.current_cell);
    }

    pub fn clear_current_cell(&mut self) {
        let index = self.index(self.current_cell());
        self.cells[index].content = CellContent::Empty;
    }

    pub fn current_cell(&self) -> (usize, usize) {
        (self.current_cell.0, self.current_cell.1)
    }

    pub fn cell_at(&self, cell_position: (usize, usize)) -> &Cell {
        let index = self.index(cell_position);
        &self.cells[index]
    }

    pub fn cell_at_mut(&mut self, cell_position: (usize, usize)) -> &mut Cell {
        let index = self.index(cell_position);
        &mut self.cells[index]
    }

    fn index(&self, cell_position: (usize, usize)) -> usize {
        cell_position.1 * self.width + cell_position.0
    }

    pub fn evaluate(&mut self) {
        let mut cells = self.cells.clone();
        for cell in &mut cells {
            cell.evaluate(self)
        }
        self.cells = cells;
    }

    pub fn serialize_as_csv(&self) -> String {
        let mut result = String::new();
        for cell in self {
            if cell.column() > self.used_cells.0 || cell.row() > self.used_cells.1 {
                continue;
            }
            if cell.column() == 0 && cell.row() != 0 {
                result.push('\n');
            }
            write!(result, "{},", cell.serialize_display_content()).unwrap();
        }
        result
    }

    pub fn save_as_xlsx(&self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        let mut spreadsheet = umya_spreadsheet::new_file();
        let worksheet = spreadsheet.get_sheet_mut(&0).unwrap();
        worksheet.set_name("Sheet!").set_active_cell(&format!(
            "{}{}",
            to_column_name(self.current_cell.0),
            self.current_cell.1 + 1
        ));
        for column in 0..self.columns() {
            // TODO: Find out if column is zero based or one based, because we
            // use it differently here then down below..
            worksheet
                .get_column_dimension_by_number_mut(&(column as u32))
                .set_width(self.column_width(column) as f64);
            for row in 0..self.rows() {
                worksheet
                    .get_cell_by_column_and_row_mut(&(column as u32 + 1), &(row as u32 + 1))
                    .set_value(self.cell_at((column, row)).content.serialize_display());
                worksheet
                    .get_style_by_column_and_row_mut(&(column as u32 + 1), &(row as u32 + 1))
                    .set_numbering_format(self.cell_at((column, row)).unit.into());
            }
        }
        umya_spreadsheet::writer::xlsx::write(&spreadsheet, path).unwrap();
    }

    pub fn recommended_cell_content(&self, position: (usize, usize)) -> CellContent {
        let from_cell = self.cell_at(position);
        let x_diff = self.current_cell().0 as isize - position.0 as isize;
        let y_diff = self.current_cell().1 as isize - position.1 as isize;
        match &from_cell.content {
            CellContent::Empty => CellContent::Empty,
            CellContent::Text(it) => CellContent::Text(it.clone()),
            CellContent::Number(it) => CellContent::Number(*it + x_diff as i64 + y_diff as i64),
            CellContent::FloatNumber(it, d) => CellContent::FloatNumber(*it, *d),
            CellContent::Formula(f) => {
                CellContent::Formula(f.moved_to(self.current_cell, (self.width, self.height)))
            }
        }
    }

    pub fn update_cell_at(&mut self, cell_position: (usize, usize), cell_content: CellContent) {
        let index = self.index(cell_position);
        self.cells[index].content = cell_content;
    }

    pub fn as_rows(&self) -> SpreadsheetRowIter {
        SpreadsheetRowIter {
            spreadsheet: self,
            index: 0,
        }
    }

    pub fn sort_column(&mut self, column: usize) {
        let rows: Vec<_> = self.as_rows().skip(self.fixed_rows).collect();
        let mut rows = rows.clone();
        rows.sort_by_cached_key(|r| &r[column].content);
        rows.reverse();
        self.cells = self
            .as_rows()
            .take(self.fixed_rows)
            .chain(rows.into_iter())
            .flatten()
            .cloned()
            .collect();
        for (index, cell) in self.cells.iter_mut().enumerate() {
            cell.position = CellPosition::from_index(index, self.width);
        }
    }

    pub fn fit_column_width(&mut self, column: usize) {
        let width = self
            .as_rows()
            .map(|r| r[column].display_content().as_ref().width())
            .fold(0, |a, w| a.max(w));
        self.set_column_width(column, width + 1);
    }

    pub fn fix_rows(&mut self, fixed_rows: usize) {
        self.fixed_rows = fixed_rows;
    }
}

impl<'a> IntoIterator for &'a Spreadsheet {
    type Item = &'a Cell;

    type IntoIter = std::slice::Iter<'a, Cell>;

    fn into_iter(self) -> Self::IntoIter {
        self.cells.iter()
    }
}

pub struct SpreadsheetRowIter<'a> {
    spreadsheet: &'a Spreadsheet,
    index: usize,
}

impl<'a> Iterator for SpreadsheetRowIter<'a> {
    type Item = &'a [Cell];

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.spreadsheet.height {
            None
        } else {
            let start = self.index * self.spreadsheet.width;
            self.index += 1;
            Some(&self.spreadsheet.cells[start..][..self.spreadsheet.width])
        }
    }
}

pub fn to_column_name(mut index: usize) -> String {
    let mut result = String::new();
    let letters = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    ];
    while index >= 26 {
        result.insert(0, letters[index % 26]);
        index /= 26;
    }
    result.insert(0, letters[index]);
    result
}

pub fn column_name_to_index(column: &str) -> Result<usize, &str> {
    let mut result = 0;
    if column.is_empty() {
        return Err(column);
    }
    for ch in column.chars() {
        if ch.is_ascii_alphabetic() {
            let ch = ch.to_ascii_uppercase();
            let digit = (ch as u8) - b'A';
            result = result * 26 + digit as usize;
        } else {
            return Err(column);
        }
    }
    Ok(result)
}

pub fn cell_name_to_position(cell: &str) -> Result<(usize, usize), &str> {
    let mut x = 0;
    let mut y = 0;
    let mut is_at_column = true;
    let mut has_column = false;
    for (i, ch) in cell.char_indices() {
        if is_at_column && ch.is_ascii_alphabetic() && ch.is_ascii_uppercase() {
            has_column = true;
            let ch = ch.to_ascii_uppercase();
            let digit = (ch as u8) - b'A';
            x = x * 26 + digit as usize;
        } else if is_at_column {
            if !has_column {
                return Err(cell);
            }
            is_at_column = false;
            y = cell[i..].parse().map_err(|_| &cell[i..])?;
            break;
        } else {
            return Err(&cell[i..]);
        }
    }
    if cell.is_empty() || is_at_column {
        Err(cell)
    } else {
        Ok((x, y))
    }
}

pub fn cell_position_to_name((x, y): (usize, usize)) -> String {
    CellPosition(x, y).name()
}
