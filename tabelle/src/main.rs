//! <!------------------------------------------------------------------------->
//!
//! [license]: https://img.shields.io/github/license/wert007/tabelle
//! [repository]: https://github.com/wert007/tabelle
//!
//! <!------------------------------------------------------------------------->
//!
//! # tabelle
//!
//! ## Summary
//!
//! [![][license]][repository]
//!
//! A simple `.csv` and `.xlsx` viewer for your terminal.
//!
//! ## Running & Commandline Args
//!
//! You can open a file by typing `tabelle file.csv` or just start a new one by
//! running `tabelle`.
//!
//! ## Features
//!
//! It supports formulas, just like any other spreadsheet program. They start
//! with an `=` and then contain python code. You can refer to columns and cells
//! by their names, both in UPPERCASE and lowercase (not mixed though!). If you
//! save as csv it will just save the value of the formula. To keep the formula
//! use the `.xlsx` format.
//!
//! ## Installation
//!
//! You need cargo installed to install this, then just execute this command:
//!
//! ```bash
//! cargo install --git https://github.com/wert007/tabelle
//! ```
//!
//! ## Contributions
//!
//! This is just a small personal project for me, at the same time I feel like
//! there is an empty niche for terminal spreadsheet viewer. I personally add
//! features, when I will need them, if you want to add features of your own
//! feel free to open an issue or a pull request. Just make sure to run `cargo
//! fmt` and `cargo clippy` before opening your pull request.

use commands::{Command, CommandKind};
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::{cursor::*, event::KeyModifiers, style::*, terminal::*, *};
use dialog::{Dialog, DialogPurpose};
use serde::{Deserialize, Serialize};
use std::io::{stdout, Write};
use std::path::PathBuf;
use strum::IntoEnumIterator;
use tabelle_core::{to_column_name, CellContent, Spreadsheet};
use text_input::TextInput;
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

mod commands;
mod dialog;
mod text_input;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    spreadsheet: Spreadsheet,
    cursor: (u16, u16),
    dialog: Option<Dialog>,
}

struct Terminal {
    width: u16,
    height: u16,
    spreadsheet: Spreadsheet,
    cursor: (u16, u16),
    dialog: Option<Dialog>,
    scroll_page: ScrollPage,
    command_line_has_focus: bool,
    command_line: TextInput,
    cell_editor: Option<TextInput>,
}

impl Terminal {
    pub fn new() -> Self {
        crossterm::terminal::enable_raw_mode().expect("Failed to enable raw mode!");
        execute!(stdout(), EnterAlternateScreen, MoveTo(0, 0))
            .expect("Failed to enter alternate screen.");
        let (width, height) =
            crossterm::terminal::size().expect("Failed to receive terminal size.");
        let config = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join("config.json");
        let mut cursor = (7, 3);
        let mut dialog = None;
        let args: Vec<String> = std::env::args().collect();
        let spreadsheet = if args.len() > 1 {
            let file: PathBuf = args[1].as_str().into();
            if file.exists() {
                if file.extension().and_then(|e| e.to_str()) == Some("xlsx") {
                    Spreadsheet::load_xlsx(file)
                } else {
                    let content = std::fs::read_to_string(&file).unwrap();
                    match Spreadsheet::load_csv(&content) {
                        Ok(it) => it,
                        Err(err) => {
                            dialog = Some(Dialog::display_error(format!(
                                "Error while opening {}: {err:?}",
                                file.display(),
                            )));
                            Spreadsheet::new(5, 5)
                        }
                    }
                }
            } else {
                Spreadsheet::new(5, 5)
            }
        } else if config.exists() {
            let config: Config =
                serde_json::from_str(&std::fs::read_to_string(config).unwrap()).unwrap();
            cursor = config.cursor;
            dialog = config.dialog;
            config.spreadsheet
        } else {
            Spreadsheet::new(5, 5)
        };
        let size = cursor_to_cell((width, height));
        let scroll_page = ScrollPage::new(spreadsheet.current_cell(), size);
        Self {
            width,
            height,
            spreadsheet,
            cursor,
            dialog,
            scroll_page,
            command_line_has_focus: false,
            command_line: TextInput::default(),
            cell_editor: None,
        }
    }

    pub fn start(&mut self) -> crossterm::Result<()> {
        self.render()?;
        loop {
            let event = crossterm::event::read()?;
            if if self.command_line_has_focus {
                self.handle_command_line_event(event)?
            } else if let Some(cell_editor) = self.cell_editor.as_mut() {
                let mut key_event = None;
                let result = handle_text_input_event(cell_editor, event, &mut key_event)?;
                match key_event {
                    Some(KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    }) => {
                        let cell_editor = self.cell_editor.take().unwrap();
                        let cell_position = self.spreadsheet.current_cell();
                        self.spreadsheet.update_cell_at(
                            cell_position,
                            CellContent::parse(
                                &cell_editor.buffer,
                                cell_position,
                                (self.spreadsheet.columns(), self.spreadsheet.rows()),
                            ),
                        );
                        self.spreadsheet.evaluate();
                        if !self.move_cursor(0, 1)? {
                            self.spreadsheet
                                .resize(self.spreadsheet.columns(), self.spreadsheet.rows() + 1);
                            self.move_cursor_force_render(0, 1)?;
                        }
                        self.update_cursor(cell_position)?;
                        self.render()?;
                        false
                    }
                    Some(KeyEvent {
                        code: KeyCode::Tab, ..
                    }) => {
                        let cell_editor = self.cell_editor.take().unwrap();
                        let cell_position = self.spreadsheet.current_cell();
                        self.spreadsheet.update_cell_at(
                            cell_position,
                            CellContent::parse(
                                &cell_editor.buffer,
                                cell_position,
                                (self.spreadsheet.columns(), self.spreadsheet.rows()),
                            ),
                        );
                        self.spreadsheet.evaluate();
                        if !self.move_cursor(1, 0)? {
                            self.spreadsheet
                                .resize(self.spreadsheet.columns() + 1, self.spreadsheet.rows());
                            self.move_cursor_force_render(1, 0)?;
                        }
                        self.update_cursor(cell_position)?;
                        self.render()?;
                        false
                    }
                    _ => {
                        self.render_status_bar()?;
                        result
                    }
                }
            } else {
                self.handle_event(event)?
            } {
                break;
            }
        }
        Ok(())
    }

    fn move_cursor(&mut self, x: isize, y: isize) -> crossterm::Result<bool> {
        if self.spreadsheet.current_cell() != self.scroll_page.no_scroll_cursor(self.cell_size()) {
            execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
            panic!(
                "scroll_page: {:#?}, cell_size: {:?}",
                self.scroll_page,
                self.cell_size()
            );
        }
        let old_cursor = self.scroll_page.cursor;
        let result = self.spreadsheet.move_cursor(x, y);
        if result {
            if self.scroll_page.move_cursor((x, y), self.cell_size()) {
                // self.render()? flushes this queue to the terminal
                queue!(stdout(), Clear(ClearType::All))?;
                self.render()?;
            } else {
                self.render_status_bar()?;
            }
        }
        self.update_cursor(old_cursor)?;
        Ok(result)
    }

    fn set_cursor(&mut self, x: usize, y: usize) -> crossterm::Result<()> {
        if self.spreadsheet.current_cell() != self.scroll_page.no_scroll_cursor(self.cell_size()) {
            execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
            panic!(
                "scroll_page: {:#?}, cell_size: {:?}",
                self.scroll_page,
                self.cell_size()
            );
        }
        let old_cursor = self.scroll_page.cursor;
        self.spreadsheet.set_cursor((x, y));
        self.scroll_page.set_cursor((x, y), self.cell_size());
        // self.render()? flushes this queue to the terminal
        queue!(stdout(), Clear(ClearType::All))?;
        self.render()?;
        self.update_cursor(old_cursor)?;
        Ok(())
    }

    fn move_cursor_force_render(&mut self, x: isize, y: isize) -> crossterm::Result<bool> {
        if self.spreadsheet.current_cell() != self.scroll_page.no_scroll_cursor(self.cell_size()) {
            execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
            panic!(
                "scroll_page: {:#?}, cell_size: {:?}",
                self.scroll_page,
                self.cell_size()
            );
        }
        let old_cursor = self.scroll_page.cursor;
        let result = self.spreadsheet.move_cursor(x, y);
        if result {
            self.scroll_page.move_cursor((x, y), self.cell_size());
            self.render()?;
        }
        self.update_cursor(old_cursor)?;
        Ok(result)
    }

    fn render_status_bar(&self) -> crossterm::Result<()> {
        let cell_position = self.spreadsheet.current_cell();
        let color = if self.spreadsheet.cell_at(cell_position).is_error() {
            Color::DarkRed
        } else {
            Color::DarkGrey
        };
        queue!(stdout(), MoveTo(0, 0), SetBackgroundColor(color))?;
        let index = format!("{}{}", to_column_name(cell_position.0), cell_position.1);
        // let content = content.unicode_pad(self.width as _,
        // unicode_truncate::Alignment::Left, true);
        let mut recommended = String::new();
        let mut cursor = (0, 1);
        let content = if let Some(cell_editor) = &self.cell_editor {
            cursor = (index.len() as u16 + 2 + cell_editor.cursor() as u16, 0);
            cell_editor.buffer.as_str().into()
        } else {
            let pos = self.spreadsheet.current_cell();
            let pos = (pos.0, pos.1.saturating_sub(1));
            recommended = self
                .spreadsheet
                .recommended_cell_content(pos)
                .serialize_display()
                .into_owned();
            self.spreadsheet
                .cell_at(cell_position)
                .long_display_content()
        };
        let available_width = self.width as usize - index.len() - 2;
        let content = content.unicode_truncate(available_width / 2 - 1).0;
        let recommended = recommended.unicode_truncate(available_width / 2 - 1).0;
        queue!(
            stdout(),
            Clear(ClearType::UntilNewLine),
            MoveToColumn(0),
            Print(index),
            Print(": "),
            Print(content),
            MoveToColumn(available_width as u16 / 2),
            Print('|'),
            Print(recommended),
            ResetColor,
            MoveTo(cursor.0, cursor.1),
        )?;
        stdout().flush()?;
        Ok(())
    }

    fn render(&self) -> crossterm::Result<()> {
        self.render_status_bar()?;
        let mut cursor = (0, 1);

        let scroll = self.scroll_page.scroll(self.cell_size());

        queue!(stdout(), ResetColor, Print("    "))?;
        for column in scroll.0..self.spreadsheet.columns() {
            let column_width = self.spreadsheet.column_width(column);
            let column = to_column_name(column);
            queue!(
                stdout(),
                Print(" │ "),
                Print(column.unicode_pad(column_width, unicode_truncate::Alignment::Left, true)),
            )?;
            cursor.0 += column_width as u16 + 3;
            if cursor.0 + column_width as u16 + 3 > self.width {
                break;
            }
        }
        queue!(stdout(), MoveRight(1), Print('│'),)?;
        for cell in &self.spreadsheet {
            if cell.column() < scroll.0 || cell.row() < scroll.1 {
                continue;
            }
            let column_width = self.spreadsheet.column_width(cell.column());
            if cell.column() == scroll.0 {
                if cell.row() != scroll.1 {
                    queue!(
                        stdout(),
                        MoveRight(2),
                        Clear(ClearType::UntilNewLine),
                        MoveDown(1),
                        Clear(ClearType::UntilNewLine),
                        MoveDown(1),
                        MoveToColumn(0)
                    )?;
                    cursor = (5, cursor.1 + 2);
                } else {
                    queue!(
                        stdout(),
                        MoveRight(2),
                        Clear(ClearType::UntilNewLine),
                        MoveDown(1),
                        MoveToColumn(0)
                    )?;
                    cursor = (5, cursor.1 + 1);
                }
                if cursor.1 + 3 > self.height {
                    break;
                }
                queue!(
                    stdout(),
                    Print("─────"),
                    MoveDown(2),
                    MoveToColumn(0),
                    Print("─────"),
                    MoveToColumn(0),
                    MoveUp(1),
                    Print(format!("{:5}", cell.row())),
                    MoveUp(1),
                )?;
            }
            if cursor.0 + column_width as u16 + 2 > self.width {
                continue;
            }
            let alignment = if cell.is_right_aligned() {
                unicode_truncate::Alignment::Right
            } else {
                unicode_truncate::Alignment::Left
            };
            let neighbors = Neighbors {
                top: true,
                right: cell.column() + 1 < self.spreadsheet.columns(),
                bottom: cell.row() + 1 < self.spreadsheet.rows(),
                left: true,
            };
            print_cell(
                cell.display_content()
                    .unicode_pad(column_width, alignment, true)
                    .as_ref(),
                cursor.0,
                neighbors,
                cell.position() == self.spreadsheet.current_cell(),
            )?;
            cursor.0 += column_width as u16 + 2 + 1;
            queue!(stdout(), MoveTo(cursor.0, cursor.1), ResetColor)?;
        }

        self.render_command_line()?;

        queue!(
            stdout(),
            SetBackgroundColor(Color::Reset),
            MoveTo(self.cursor.0, self.cursor.1)
        )?;

        stdout().flush()?;
        if let Some(dialog) = &self.dialog {
            dialog.render()?;
        }

        Ok(())
    }

    fn render_help(&self) -> crossterm::Result<()> {
        queue!(
            stdout(),
            Clear(ClearType::All),
            MoveTo(0, 0),
            Print("Help"),
            MoveToNextLine(1)
        )?;
        for cmd in CommandKind::iter() {
            if cmd == CommandKind::None {
                continue;
            }
            queue!(
                stdout(),
                Print("    - "),
                Print(
                    cmd.to_string()
                        .unicode_pad(10, unicode_truncate::Alignment::Left, false)
                ),
                Print(": "),
                Print(cmd.description()),
                MoveToNextLine(1),
            )?;
            for example in cmd.example_values() {
                queue!(
                    stdout(),
                    MoveToColumn(8),
                    SetForegroundColor(Color::DarkGrey),
                    Print("Example :  "),
                    Print(example.full_display()),
                    MoveToNextLine(1),
                    ResetColor,
                )?;
            }
        }
        queue!(
            stdout(),
            MoveToNextLine(1),
            Print("Press ENTER to exit help. Press ESC to exit this program."),
        )?;
        self.render_command_line()?;
        Ok(())
    }

    fn render_command_line(&self) -> crossterm::Result<()> {
        queue!(
            stdout(),
            MoveTo(0, self.width - 1),
            SetBackgroundColor(Color::DarkGreen),
        )?;
        if !self.command_line_has_focus {
            queue!(stdout(), Print("Press Ctrl+X to enter command line"))?;
        } else {
            queue!(stdout(), Print("> "), Print(&self.command_line.buffer),)?;
        };
        queue!(stdout(), Clear(ClearType::UntilNewLine), ResetColor)?;
        stdout().flush()?;

        if self.command_line_has_focus {
            queue!(
                stdout(),
                MoveToColumn(self.command_line.cursor() as u16 + 2)
            )?;
        }

        stdout().flush()?;
        Ok(())
    }

    fn update_cursor(&mut self, old_cursor: (usize, usize)) -> crossterm::Result<()> {
        if self.spreadsheet.current_cell() != self.scroll_page.no_scroll_cursor(self.cell_size()) {
            execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
            println!(
                "scroll_page: {:#?}, cell_size: {:?}",
                self.scroll_page,
                self.cell_size()
            );
        }
        assert_eq!(
            self.spreadsheet.current_cell(),
            self.scroll_page.no_scroll_cursor(self.cell_size()),
        );
        self.update_highlighted_cell(old_cursor, self.scroll_page.cursor)?;
        let cursor = self.cell_to_cursor(self.scroll_page.cursor);
        self.cursor = cursor;
        execute!(stdout(), MoveTo(self.cursor.0, self.cursor.1))
    }

    fn cell_to_cursor(&self, cell_position: (usize, usize)) -> (u16, u16) {
        let offset = (7, 3);
        let height_per_cell = 2;
        let width: usize = (0..cell_position.0)
            .map(|c| self.spreadsheet.column_width(c) + 3)
            .sum();
        // let size = cursor_to_cell((self.width, self.height));
        // let scroll = self.scroll_page.scroll(size);
        let x = offset.0 + width as u16;
        let y = offset.1 + height_per_cell * cell_position.1 as u16;
        (x, y)
    }

    fn cell_size(&self) -> (usize, usize) {
        let result = cursor_to_cell((self.width - 1, self.height - 1));
        (result.0 - 1, result.1 - 1)
    }

    fn handle_event(&mut self, event: event::Event) -> crossterm::Result<bool> {
        match event {
            crossterm::event::Event::FocusGained => {}
            crossterm::event::Event::FocusLost => {}
            crossterm::event::Event::Key(key) => {
                if let Some(dialog) = &mut self.dialog {
                    match dialog.update(key)? {
                        dialog::DialogResult::None => {}
                        dialog::DialogResult::Close => self.dialog = None,
                        dialog::DialogResult::Yes(_) => match dialog.purpose() {
                            DialogPurpose::CommandOutput => {
                                self.dialog = None;
                            }
                        },
                    }
                    Dialog::clear(8)?;
                    if let Some(dialog) = &self.dialog {
                        dialog.render()?;
                    } else {
                        self.render()?;
                    }
                } else {
                    match key.code {
                        crossterm::event::KeyCode::Backspace => {
                            self.spreadsheet.clear_current_cell();
                            self.render()?;
                        }
                        crossterm::event::KeyCode::Enter => {
                            let cell_position = self.spreadsheet.current_cell();
                            let cell = self.spreadsheet.cell_at(cell_position);
                            let text = if cell.is_empty() && cell_position.1 > 0 {
                                self.spreadsheet
                                    .recommended_cell_content((
                                        cell_position.0,
                                        cell_position.1 - 1,
                                    ))
                                    .serialize_display()
                                    .into_owned()
                            } else {
                                cell.serialize_display_content().into_owned()
                            };
                            self.init_cell_editor(text)?;
                        }
                        crossterm::event::KeyCode::Left => {
                            self.move_cursor(-1, 0)?;
                        }
                        crossterm::event::KeyCode::Right => {
                            self.move_cursor(1, 0)?;
                        }
                        crossterm::event::KeyCode::Up => {
                            self.move_cursor(0, -1)?;
                        }
                        crossterm::event::KeyCode::Down => {
                            self.move_cursor(0, 1)?;
                        }
                        crossterm::event::KeyCode::Home => self.set_cursor(0, 0)?,
                        crossterm::event::KeyCode::End => self.set_cursor(
                            self.spreadsheet.columns() - 1,
                            self.spreadsheet.rows() - 1,
                        )?,
                        crossterm::event::KeyCode::PageUp => {
                            self.move_cursor(0, -(self.cell_size().1 as isize))?;
                        }
                        crossterm::event::KeyCode::PageDown => {
                            self.move_cursor(0, self.cell_size().1 as isize)?;
                        }
                        crossterm::event::KeyCode::Tab => {
                            let old_cursor = self.scroll_page.cursor;
                            if !self.move_cursor(1, 0)? {
                                self.spreadsheet.resize(
                                    self.spreadsheet.columns() + 1,
                                    self.spreadsheet.rows(),
                                );

                                self.move_cursor_force_render(1, 0)?;
                                self.render()?;
                            }
                            self.update_cursor(old_cursor)?;
                        }
                        crossterm::event::KeyCode::BackTab => {
                            self.move_cursor(-1, 0)?;
                        }
                        crossterm::event::KeyCode::Delete => {
                            self.spreadsheet.clear_current_cell();
                            self.render()?;
                        }
                        crossterm::event::KeyCode::Insert => {}
                        crossterm::event::KeyCode::F(1) => {
                            self.command_line_has_focus = true;
                            self.command_line.set("");
                            self.render_command_line()?;
                            self.render_help()?;
                        }
                        crossterm::event::KeyCode::F(_) => {}
                        crossterm::event::KeyCode::Char('d' | 'c')
                            if key.modifiers == KeyModifiers::CONTROL =>
                        {
                            return Ok(true);
                        }
                        crossterm::event::KeyCode::Char('r')
                            if key.modifiers == KeyModifiers::CONTROL =>
                        {
                            self.command_line_has_focus = true;
                            self.command_line.set("resize ");
                            self.render_command_line()?;
                        }
                        crossterm::event::KeyCode::Char('s')
                            if key.modifiers == KeyModifiers::CONTROL =>
                        {
                            self.command_line_has_focus = true;
                            self.command_line.set(&format!(
                                "save {}",
                                self.spreadsheet
                                    .path()
                                    .map(|p| p.display())
                                    .unwrap_or_else(|| std::path::Path::new(".xlsx").display())
                            ));
                            self.command_line.set_cursor(5);
                            self.render_command_line()?;
                        }
                        crossterm::event::KeyCode::Char('g')
                            if key.modifiers == KeyModifiers::CONTROL =>
                        {
                            self.command_line_has_focus = true;
                            self.command_line.set("goto ");
                            self.render_command_line()?;
                        }
                        crossterm::event::KeyCode::Char('x')
                            if key.modifiers == KeyModifiers::CONTROL =>
                        {
                            self.command_line_has_focus = true;
                            // Faster then self.render()?;
                            self.render_command_line()?;
                        }
                        crossterm::event::KeyCode::Char('f')
                            if key.modifiers == KeyModifiers::CONTROL =>
                        {
                            self.command_line_has_focus = true;
                            self.command_line.set("find ");
                            self.render_command_line()?;
                        }
                        crossterm::event::KeyCode::Char(ch) => {
                            self.init_cell_editor(ch.to_string())?;
                        }
                        crossterm::event::KeyCode::Null => return Ok(true),
                        crossterm::event::KeyCode::Esc => {
                            return Ok(true);
                        }
                        crossterm::event::KeyCode::CapsLock => {}
                        crossterm::event::KeyCode::ScrollLock => {}
                        crossterm::event::KeyCode::NumLock => {}
                        crossterm::event::KeyCode::PrintScreen => {}
                        crossterm::event::KeyCode::Pause => {}
                        crossterm::event::KeyCode::Menu => {}
                        crossterm::event::KeyCode::KeypadBegin => {}
                        crossterm::event::KeyCode::Media(_) => {}
                        crossterm::event::KeyCode::Modifier(_) => {}
                    }
                }
            }
            crossterm::event::Event::Mouse(_) => {}
            crossterm::event::Event::Paste(_) => {}
            crossterm::event::Event::Resize(width, height) => {
                self.width = width;
                self.height = height;
            }
        }
        Ok(false)
    }

    fn handle_command_line_event(&mut self, event: event::Event) -> crossterm::Result<bool> {
        match event {
            event::Event::FocusGained => {}
            event::Event::FocusLost => {}
            event::Event::Key(key_event) => match key_event.code {
                event::KeyCode::Backspace => self.command_line.backspace(),
                event::KeyCode::Enter => {
                    let command = match Command::parse(&self.command_line.buffer) {
                        Ok(it) => it,
                        Err(_) => return Ok(false),
                    };
                    self.command_line.clear();
                    if command.execute(self)? {
                        self.command_line_has_focus = false;
                        // self.update_cursor()?;
                        self.render()?;
                    }
                }
                event::KeyCode::Left => self.command_line.left(),
                event::KeyCode::Right => self.command_line.right(),
                event::KeyCode::Up => self.command_line.up(),
                event::KeyCode::Down => self.command_line.down(),
                event::KeyCode::Home => self.command_line.up(),
                event::KeyCode::End => self.command_line.down(),
                event::KeyCode::PageUp => {}
                event::KeyCode::PageDown => {}
                event::KeyCode::Tab => {}
                event::KeyCode::BackTab => {}
                event::KeyCode::Delete => self.command_line.delete(),
                event::KeyCode::Insert => {}
                event::KeyCode::F(_) => {}
                event::KeyCode::Char(ch) => self.command_line.insert_char(ch),
                event::KeyCode::Null | event::KeyCode::Esc => return Ok(true),
                event::KeyCode::CapsLock => {}
                event::KeyCode::ScrollLock => {}
                event::KeyCode::NumLock => {}
                event::KeyCode::PrintScreen => {}
                event::KeyCode::Pause => {}
                event::KeyCode::Menu => {}
                event::KeyCode::KeypadBegin => {}
                event::KeyCode::Media(_) => {}
                event::KeyCode::Modifier(_) => {}
            },
            event::Event::Mouse(_) => {}
            event::Event::Paste(_) => {}
            event::Event::Resize(_, _) => {}
        }
        self.render_command_line()?;
        Ok(false)
    }

    fn init_cell_editor(&mut self, text: String) -> crossterm::Result<()> {
        let mut cell_editor = TextInput::default();
        cell_editor.set(&text);
        self.cell_editor = Some(cell_editor);
        self.render_status_bar()?;
        Ok(())
    }

    pub(crate) fn update_highlighted_cell(
        &self,
        old_cursor: (usize, usize),
        new_cursor: (usize, usize),
    ) -> crossterm::Result<()> {
        let size = self.cell_size();
        let size = (
            size.0.min(self.spreadsheet.columns()),
            size.1.min(self.spreadsheet.rows()),
        );
        let neighbors = Neighbors {
            top: true,
            right: old_cursor.0 + 1 < size.0,
            bottom: old_cursor.1 + 1 < size.1,
            left: true,
        };
        let width = self.spreadsheet.column_width(old_cursor.0) as u16;
        let cursor = self.cell_to_cursor(old_cursor);
        let cursor = (cursor.0 - 2, cursor.1 - 1);
        print_cell_border(cursor, width, neighbors, false)?;
        let neighbors = Neighbors {
            top: true,
            right: new_cursor.0 + 1 < size.0,
            bottom: new_cursor.1 + 1 < size.1,
            left: true,
        };
        let cursor = self.cell_to_cursor(new_cursor);
        let cursor = (cursor.0 - 2, cursor.1 - 1);
        let width = self.spreadsheet.column_width(new_cursor.0) as u16;
        print_cell_border(cursor, width, neighbors, true)?;
        Ok(())
    }
}

fn handle_text_input_event(
    input: &mut TextInput,
    event: event::Event,
    unhandled_key_event: &mut Option<KeyEvent>,
) -> crossterm::Result<bool> {
    match event {
        event::Event::FocusGained => {}
        event::Event::FocusLost => {}
        event::Event::Key(event) => match event.code {
            event::KeyCode::Backspace => input.backspace(),
            event::KeyCode::Tab | event::KeyCode::Enter => {
                *unhandled_key_event = Some(event);
            }
            event::KeyCode::Left => input.left(),
            event::KeyCode::Right => input.right(),
            event::KeyCode::Up => input.up(),
            event::KeyCode::Down => input.down(),
            event::KeyCode::Home => input.up(),
            event::KeyCode::End => input.down(),
            event::KeyCode::PageUp => {}
            event::KeyCode::PageDown => {}
            event::KeyCode::BackTab => {}
            event::KeyCode::Delete => input.delete(),
            event::KeyCode::Insert => {}
            event::KeyCode::F(_) => {}
            event::KeyCode::Char(ch) => input.insert_char(ch),
            event::KeyCode::Null | event::KeyCode::Esc => return Ok(true),
            event::KeyCode::CapsLock => {}
            event::KeyCode::ScrollLock => {}
            event::KeyCode::NumLock => {}
            event::KeyCode::PrintScreen => {}
            event::KeyCode::Pause => {}
            event::KeyCode::Menu => {}
            event::KeyCode::KeypadBegin => {}
            event::KeyCode::Media(_) => {}
            event::KeyCode::Modifier(_) => {}
        },
        event::Event::Mouse(_) => {}
        event::Event::Paste(_) => {}
        event::Event::Resize(_, _) => {}
    }
    Ok(false)
}

fn cursor_to_cell(cursor: (u16, u16)) -> (usize, usize) {
    let offset = (7, 3);
    // TODO: Fix for variable cell size.
    let size_per_cell = (12, 2);
    let x = (cursor.0 - offset.0) / size_per_cell.0;
    let y = (cursor.1 - offset.1) / size_per_cell.1;
    (x as usize, y as usize)
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let config_path = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join("config.json");
        let config = Config {
            spreadsheet: self.spreadsheet.clone(),
            cursor: self.cursor,
            dialog: self.dialog.clone(),
        };

        std::fs::write(
            config_path,
            serde_json::to_string_pretty(&config).expect("Failed to convert to json?"),
        )
        .expect("Failed to write config!");
        // execute!(stdout(), ResetColor, LeaveAlternateScreen)
        //     .expect("Failed to leave alternate screen.");
        crossterm::terminal::disable_raw_mode().expect("Failed to disable raw mode!");
    }
}

#[derive(Debug)]
struct ScrollPage {
    scroll_page: (usize, usize),
    cursor: (usize, usize),
}

impl ScrollPage {
    pub fn new(mut cursor: (usize, usize), size: (usize, usize)) -> ScrollPage {
        let mut scroll_page = (0, 0);
        while cursor.0 > size.0 {
            scroll_page.0 += 1;
            cursor.0 -= size.0;
        }
        while cursor.1 > size.1 {
            scroll_page.1 += 1;
            cursor.1 -= size.1;
        }
        ScrollPage {
            scroll_page,
            cursor,
        }
    }

    pub fn move_cursor(&mut self, offset: (isize, isize), size: (usize, usize)) -> bool {
        let mut result = false;
        let mut cursor = (
            self.cursor.0 as isize + offset.0,
            self.cursor.1 as isize + offset.1,
        );

        if cursor.0 < 0 {
            if self.scroll_page.0 > 0 {
                result = true;
                self.scroll_page.0 -= 1;
                cursor.0 += size.0 as isize;
            } else {
                cursor.0 = 0;
            }
        }
        if cursor.1 < 0 {
            if self.scroll_page.1 > 0 {
                result = true;
                self.scroll_page.1 -= 1;
                cursor.1 += size.1 as isize;
            } else {
                cursor.1 = 0;
            }
        }
        let mut cursor = (cursor.0 as usize, cursor.1 as usize);
        while cursor.0 >= size.0 {
            result = true;
            self.scroll_page.0 += 1;
            cursor.0 -= size.0;
        }
        while cursor.1 >= size.1 {
            result = true;
            self.scroll_page.1 += 1;
            cursor.1 -= size.1;
        }
        self.cursor = cursor;

        result
    }

    fn scroll(&self, size: (usize, usize)) -> (usize, usize) {
        (self.scroll_page.0 * size.0, self.scroll_page.1 * size.1)
    }

    fn no_scroll_cursor(&self, size: (usize, usize)) -> (usize, usize) {
        (
            self.scroll_page.0 * size.0 + self.cursor.0,
            self.scroll_page.1 * size.1 + self.cursor.1,
        )
    }

    fn set_cursor(&mut self, cursor: (usize, usize), size: (usize, usize)) {
        self.cursor = cursor;
        while self.cursor.0 > size.0 {
            self.scroll_page.0 += 1;
            self.cursor.0 -= size.0;
        }
        while self.cursor.1 > size.1 {
            self.scroll_page.1 += 1;
            self.cursor.1 -= size.1;
        }
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

fn print_cell_border(
    cursor: (u16, u16),
    width: u16,
    neighbors: Neighbors,
    highlight: bool,
) -> crossterm::Result<()> {
    let color = if highlight { Color::Cyan } else { Color::Reset };
    queue!(
        stdout(),
        SetForegroundColor(color),
        MoveTo(cursor.0, cursor.1),
        Print(neighbors.top_left_char())
    )?;
    for _ in 0..width + 2 {
        queue!(stdout(), Print('─'))?;
    }
    queue!(
        stdout(),
        Print(neighbors.top_right_char()),
        MoveDown(1),
        MoveToColumn(cursor.0),
        Print("│ "),
        MoveRight(width),
        Print(" │"),
        MoveDown(1),
        MoveToColumn(cursor.0),
        Print(neighbors.bottom_left_char())
    )?;
    for _ in 0..width + 2 {
        queue!(stdout(), Print('─'))?;
    }
    queue!(
        stdout(),
        Print(neighbors.bottom_right_char()),
        SetForegroundColor(Color::Reset)
    )?;
    // stdout().flush()?;
    Ok(())
}

fn print_cell(
    content: &str,
    cursor_column: u16,
    neighbors: Neighbors,
    highlight: bool,
) -> crossterm::Result<()> {
    let width = content.width();
    queue!(stdout(), Print(neighbors.top_left_char()))?;
    for _ in 0..width + 2 {
        queue!(stdout(), Print('─'))?;
    }
    queue!(
        stdout(),
        Print(neighbors.top_right_char()),
        MoveDown(1),
        MoveToColumn(cursor_column),
        Print("│ "),
        if highlight {
            Print(content.italic())
        } else {
            Print(content.stylize())
        },
        Print(" │"),
        MoveDown(1),
        MoveToColumn(cursor_column),
        Print(neighbors.bottom_left_char())
    )?;
    for _ in 0..width + 2 {
        queue!(stdout(), Print('─'))?;
    }
    queue!(stdout(), Print(neighbors.bottom_right_char()))?;
    // stdout().flush()?;
    Ok(())
}

fn print_blank_line(len: usize) {
    for _ in 0..len {
        print!(" ");
    }
    println!();
}

fn main() {
    // tabelle_core::dump("units-test.xlsx");
    let mut terminal = Terminal::new();
    let _ = terminal.start();
}
