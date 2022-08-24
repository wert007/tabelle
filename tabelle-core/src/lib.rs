use cells::{cell_content::CellContent, Cell, CellPosition};
use serde::{Deserialize, Serialize};
use std::{fmt::Write, path::Path};
mod cells;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spreadsheet {
    current_cell: CellPosition,
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    used_cells: CellPosition,
}

impl Spreadsheet {
    pub fn new(width: usize, height: usize) -> Self {
        let mut cells = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                cells.push(Cell {
                    content: CellContent::default(),
                    position: CellPosition(x, y),
                });
            }
        }
        Self {
            current_cell: CellPosition(0, 0),
            width,
            height,
            cells,
            used_cells: CellPosition(0, 0),
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
                    content: CellContent::parse(cell, CellPosition(column, row)),
                    position: CellPosition(column, row),
                });
                column += 1;
            }
            width = width.max(column);
            column = 0;
            row += 1;
        }
        let height = row;
        let mut result = Self {
            current_cell: CellPosition(0, 0),
            width,
            height,
            cells,
            used_cells: CellPosition(0, 0),
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
        for y in 0..height {
            for x in 0..width {
                let content = if let Some(cell) =
                    worksheet.get_cell_by_column_and_row(&((x as u32) + 1), &((y as u32) + 1))
                {
                    CellContent::parse(&cell.get_value(), CellPosition(x, y))
                } else {
                    CellContent::Empty
                };
                cells.push(Cell {
                    content,
                    position: CellPosition(x, y),
                })
            }
        }
        let mut result = Self {
            current_cell,
            width,
            height,
            cells,
            used_cells: CellPosition(width, height),
        };
        // This is very brute forcey. Could be fixed probably.
        for _ in 0..width * height {
            result.evaluate()
        }
        result
    }

    pub fn columns(&self) -> usize {
        self.width
    }

    pub fn rows(&self) -> usize {
        self.height
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        let additional = width * height - self.cells.len();
        self.cells.reserve(additional);
        for x in 0..width {
            for y in self.height..height {
                self.cells.push(Cell {
                    content: CellContent::Empty,
                    position: CellPosition(x, y),
                });
            }
        }
        for x in self.width..width {
            for y in 0..self.height {
                self.cells.push(Cell {
                    content: CellContent::Empty,
                    position: CellPosition(x, y),
                });
            }
        }
        self.cells.sort();
        self.width = width;
        self.height = height;
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
        let index = self.current_cell.1 * self.width + self.current_cell.0;
        self.cells[index].content.input_char(ch, self.current_cell);
    }

    pub fn clear_current_cell(&mut self) {
        let index = self.current_cell.1 * self.width + self.current_cell.0;
        self.cells[index].content = CellContent::Empty;
    }

    pub fn current_cell(&self) -> (usize, usize) {
        (self.current_cell.0, self.current_cell.1)
    }

    pub fn cell_at(&self, cell_position: (usize, usize)) -> &Cell {
        let index = cell_position.1 * self.width + cell_position.0;
        &self.cells[index]
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
            for row in 0..self.rows() {
                worksheet
                    .get_cell_by_column_and_row_mut(&(column as u32 + 1), &(row as u32 + 1))
                    .set_value(self.cell_at((column, row)).content.serialize_display());
            }
        }
        umya_spreadsheet::writer::xlsx::write(&spreadsheet, path).unwrap();
    }

    pub fn recommended_cell_content(&self) -> Option<CellContent> {
        if self.current_cell().1 == 0 {
            None
        } else {
            let above_cell = self.cell_at((self.current_cell().0, self.current_cell().1 - 1));
            match &above_cell.content {
                CellContent::Empty => None,
                CellContent::Text(it) => Some(CellContent::Text(it.clone())),
                CellContent::Number(it) => Some(CellContent::Number(*it + 1)),
                CellContent::FloatNumber(it, d) => Some(CellContent::FloatNumber(*it, *d)),
                CellContent::Formula(f) => Some(CellContent::Formula(f.clone())),
            }
        }
    }

    pub fn update_cell_at(&mut self, cell_position: (usize, usize), cell_content: CellContent) {
        let index = cell_position.1 * self.width + cell_position.0;
        self.cells[index].content = cell_content;
    }
}

impl<'a> IntoIterator for &'a Spreadsheet {
    type Item = &'a Cell;

    type IntoIter = std::slice::Iter<'a, Cell>;

    fn into_iter(self) -> Self::IntoIter {
        self.cells.iter()
    }
}

pub fn to_column_name(mut index: usize) -> String {
    let mut result = String::new();
    let letters = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    ];
    while index >= 26 {
        result.push(letters[index % 26]);
        index /= 26;
    }
    result.push(letters[index]);
    result
}
