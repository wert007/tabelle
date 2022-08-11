use crossterm::cursor::*;
use crossterm::style::*;
use crossterm::*;
use crossterm::{event::KeyModifiers, terminal::*};
use pad::PadStr;
use std::io::stdout;
use tabelle_core::Spreadsheet;

struct Terminal {
    width: u16,
    height: u16,
    spreadsheet: Spreadsheet,
    cursor: (u16, u16),
}

impl Terminal {
    pub fn new() -> Self {
        crossterm::terminal::enable_raw_mode().expect("Failed to enable raw mode!");
        execute!(stdout(), EnterAlternateScreen, MoveTo(0, 0))
            .expect("Failed to enter alternate screen.");
        let (width, height) =
            crossterm::terminal::size().expect("Failed to receive terminal size.");
        Self {
            width,
            height,
            spreadsheet: Spreadsheet::new(5, 5),
            cursor: (7, 3),
        }
    }

    pub fn start(&mut self) -> crossterm::Result<()> {
        self.render()?;
        loop {
            let event = crossterm::event::read()?;
            match event {
                crossterm::event::Event::FocusGained => {}
                crossterm::event::Event::FocusLost => {}
                crossterm::event::Event::Key(key) => match key.code {
                    crossterm::event::KeyCode::Backspace => {
                        self.spreadsheet.clear_current_cell();
                        self.render()?;
                    }
                    crossterm::event::KeyCode::Enter => {
                        if !self.spreadsheet.move_cursor(0, 1) {
                            self.spreadsheet.resize(self.spreadsheet.columns(), self.spreadsheet.rows() + 1);
                            self.spreadsheet.move_cursor(0, 1);
                            self.render()?;
                        }
                        self.update_cursor()?;
                    }
                    crossterm::event::KeyCode::Left => {
                        self.spreadsheet.move_cursor(-1, 0);
                        self.update_cursor()?;
                    }
                    crossterm::event::KeyCode::Right => {
                        self.spreadsheet.move_cursor(1, 0);
                        self.update_cursor()?;
                    }
                    crossterm::event::KeyCode::Up => {
                        self.spreadsheet.move_cursor(0, -1);
                        self.update_cursor()?;
                    }
                    crossterm::event::KeyCode::Down => {
                        self.spreadsheet.move_cursor(0, 1);
                        self.update_cursor()?;
                    }
                    crossterm::event::KeyCode::Home => todo!(),
                    crossterm::event::KeyCode::End => todo!(),
                    crossterm::event::KeyCode::PageUp => todo!(),
                    crossterm::event::KeyCode::PageDown => todo!(),
                    crossterm::event::KeyCode::Tab => {
                        if !self.spreadsheet.move_cursor(1, 0) {
                            self.spreadsheet.resize(self.spreadsheet.columns() + 1, self.spreadsheet.rows());
                            self.spreadsheet.move_cursor(1, 0);
                            self.render()?;
                        }
                        self.update_cursor()?;
                    }
                    crossterm::event::KeyCode::BackTab => {
                        self.spreadsheet.move_cursor(-1, 0);
                        self.update_cursor()?;
                    }
                    crossterm::event::KeyCode::Delete => {
                        self.spreadsheet.clear_current_cell();
                        self.render()?;
                    }
                    crossterm::event::KeyCode::Insert => todo!(),
                    crossterm::event::KeyCode::F(_) => todo!(),
                    crossterm::event::KeyCode::Char('d' | 'c')
                        if key.modifiers == KeyModifiers::CONTROL =>
                    {
                        break;
                    }
                    crossterm::event::KeyCode::Char('r')
                        if key.modifiers == KeyModifiers::CONTROL =>
                    {
                        self.spreadsheet
                            .resize(self.spreadsheet.columns() * 2, self.spreadsheet.rows() * 2);
                        self.render()?;
                    }
                    crossterm::event::KeyCode::Char(ch) => {
                        self.spreadsheet.input_char(ch);
                        self.spreadsheet.evaluate();
                        self.render()?;
                    }
                    crossterm::event::KeyCode::Null => break,
                    crossterm::event::KeyCode::Esc => break,
                    crossterm::event::KeyCode::CapsLock => todo!(),
                    crossterm::event::KeyCode::ScrollLock => todo!(),
                    crossterm::event::KeyCode::NumLock => todo!(),
                    crossterm::event::KeyCode::PrintScreen => todo!(),
                    crossterm::event::KeyCode::Pause => todo!(),
                    crossterm::event::KeyCode::Menu => todo!(),
                    crossterm::event::KeyCode::KeypadBegin => todo!(),
                    crossterm::event::KeyCode::Media(_) => todo!(),
                    crossterm::event::KeyCode::Modifier(_) => todo!(),
                },
                crossterm::event::Event::Mouse(_) => {}
                crossterm::event::Event::Paste(_) => {}
                crossterm::event::Event::Resize(width, height) => {
                    self.width = width;
                    self.height = height;
                }
            }
        }
        Ok(())
    }

    fn render_status_bar(&self) -> crossterm::Result<()> {
        let color = if self
            .spreadsheet
            .cell_at(self.spreadsheet.current_cell())
            .is_error()
        {
            Color::DarkRed
        } else {
            Color::DarkGrey
        };
        execute!(stdout(), MoveTo(0, 0), SetBackgroundColor(color))?;
        let content = format!(
            "{}{}: {}",
            (self.spreadsheet.current_cell().0 as u8 + b'A') as char,
            self.spreadsheet.current_cell().1 + 1,
            self.spreadsheet
                .cell_at(self.spreadsheet.current_cell())
                .long_display_content(),
        );
        print!("{content}");
        print_blank_line(self.width as usize - content.len());
        Ok(())
    }

    fn render(&self) -> crossterm::Result<()> {
        self.render_status_bar()?;
        let mut cursor = (0, 1);

        execute!(stdout(), ResetColor, Print("     "))?;
        for column in (0u8..self.spreadsheet.columns() as u8).map(|i| ('A' as u8 + i) as char) {
            execute!(
                stdout(),
                Print("| "),
                Print(format!("{column}").pad(10, ' ', pad::Alignment::Left, true)),
            )?;
        }
        for cell in &self.spreadsheet {
            if cell.column() == 0 {
                if cell.row() != 0 {
                    execute!(stdout(), MoveDown(2), MoveToColumn(0))?;
                    cursor = (5, cursor.1 + 2);
                } else {
                    execute!(stdout(), MoveDown(1), MoveToColumn(0))?;
                    cursor = (5, cursor.1 + 1);
                }
                execute!(
                    stdout(),
                    MoveDown(1),
                    Print(format!("{:5}", cell.row() + 1)),
                    MoveUp(1)
                )?;
            }
            let alignment = if cell.is_right_aligned() {
                pad::Alignment::Right
            } else {
                pad::Alignment::Left
            };
            let neighbors = Neighbors {
                top: cell.row() > 0,
                right: cell.column() + 1 < self.spreadsheet.columns(),
                bottom: cell.row() + 1 < self.spreadsheet.rows(),
                left: cell.column() > 0,
            };
            print_cell(
                cell.display_content().pad(10, ' ', alignment, true),
                cursor.0,
                neighbors,
                cell.position() == self.spreadsheet.current_cell(),
            )?;
            cursor.0 += 12;
            execute!(stdout(), MoveTo(cursor.0, cursor.1), ResetColor)?;
        }
        // println!("|");
        // print_horizontal_line(self.spreadsheet.columns() * 12 + 6);

        execute!(
            stdout(),
            SetBackgroundColor(Color::Reset),
            MoveTo(self.cursor.0, self.cursor.1)
        )?;
        Ok(())
    }

    fn update_cursor(&mut self) -> crossterm::Result<()> {
        self.render_status_bar()?;
        let cursor = self.cell_to_cursor(self.spreadsheet.current_cell());
        self.cursor = cursor;
        execute!(stdout(), MoveTo(self.cursor.0, self.cursor.1))
    }

    fn cell_to_cursor(&self, cell_position: (usize, usize)) -> (u16, u16) {
        let offset = (7, 3);
        let size_per_cell = (12, 2);
        let x = offset.0 + size_per_cell.0 * cell_position.0 as u16;
        let y = offset.1 + size_per_cell.1 * cell_position.1 as u16;
        (x, y)
    }
}

struct Neighbors {
    top: bool,
    right: bool,
    bottom: bool,
    left: bool,
}

impl Neighbors {
    fn top_left_char(&self) -> char {
        match (self.top, self.left) {
            (true, true) => '┼',
            (true, false) => '├',
            (false, true) => '┬',
            (false, false) => '┌',
        }
    }

    fn top_right_char(&self) -> char {
        match (self.top, self.right) {
            (true, true) => '┼',
            (true, false) => '┤',
            (false, true) => '┬',
            (false, false) => '┐',
        }
    }

    fn bottom_left_char(&self) -> char {
        match (self.bottom, self.left) {
            (true, true) => '┼',
            (true, false) => '├',
            (false, true) => '┴',
            (false, false) => '└',
        }
    }

    fn bottom_right_char(&self) -> char {
        match (self.bottom, self.right) {
            (true, true) => '┼',
            (true, false) => '┤',
            (false, true) => '┴',
            (false, false) => '┘',
        }
    }
}

fn print_cell(
    content: String,
    cursor_column: u16,
    neighbors: Neighbors,
    highlight: bool,
) -> crossterm::Result<()> {
    let width = content.len();
    print!("{}", neighbors.top_left_char());
    for _ in 0..width + 2 {
        print!("─");
    }
    print!("{}", neighbors.top_right_char());
    execute!(stdout(), MoveDown(1), MoveToColumn(cursor_column))?;
    print!("│ ");
    if highlight {
        print!("{}", content.italic());
    } else {
        print!("{}", content);
    }
    print!(" │");
    execute!(stdout(), MoveDown(1), MoveToColumn(cursor_column))?;
    print!("{}", neighbors.bottom_left_char());
    for _ in 0..width + 2 {
        print!("─");
    }
    print!("{}", neighbors.bottom_right_char());
    Ok(())
}

fn print_blank_line(len: usize) {
    for _ in 0..len {
        print!(" ");
    }
    println!();
}

impl Drop for Terminal {
    fn drop(&mut self) {
        execute!(stdout(), LeaveAlternateScreen).expect("Failed to enter alternate screen.");
        crossterm::terminal::disable_raw_mode().expect("Failed to disable raw mode!");
    }
}

fn main() {
    let mut terminal = Terminal::new();
    let _ = terminal.start();
}
