use cells::{CellPosition, Cell, cell_content::CellContent};

mod cells;

pub struct Spreadsheet {
    current_cell: CellPosition,
    width: usize,
    height: usize,
    cells: Vec<Cell>,
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
        }
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
        let index = self.current_cell.1 * self.width + self.current_cell.0;
        self.cells[index].content.input_char(ch);
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
            cell.evaluate(&self)
        }
        self.cells = cells;
    }
}

impl<'a> IntoIterator for &'a Spreadsheet {
    type Item = &'a Cell;

    type IntoIter = std::slice::Iter<'a, Cell>;

    fn into_iter(self) -> Self::IntoIter {
        self.cells.iter()
    }
}
