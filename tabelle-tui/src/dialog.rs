use std::{
    fmt::{Debug, Write},
    io::stdout,
};

use crossterm::{
    cursor::{MoveDown, MoveTo, MoveToColumn},
    event::{KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor},
    terminal,
};
use pad::PadStr;
use serde::{Deserialize, Serialize};

use crate::print_blank_line;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DialogPurpose {
    Save,
    Execute,
    CommandOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dialog {
    pub purpose: DialogPurpose,
    pub message: String,
    pub buffer: Option<String>,
    pub background_color: Color,
    pub answers: DialogAnswers,
    pub selected_answer: usize,
    pub height: usize,
}

impl Dialog {
    pub fn help_command(commands: &[&str]) -> Self {
        let mut message: String =
            "Here is a list of all commands. Type help <command> for more info.\n".into();
        for command in commands {
            writeln!(message, "    - {command}").unwrap();
        }
        Self {
            purpose: DialogPurpose::CommandOutput,
            message,
            buffer: None,
            background_color: Color::DarkYellow,
            answers: DialogAnswers::Ok,
            selected_answer: 0,
            height: 5 + commands.len(),
        }
    }

    pub fn unknown_command(unknown: String) -> Self {
        let message =
            format!("No command named {unknown} known. Type help to see all available commands.");
        Self {
            purpose: DialogPurpose::CommandOutput,
            message,
            buffer: None,
            background_color: Color::DarkYellow,
            answers: DialogAnswers::Ok,
            selected_answer: 0,
            height: 5,
        }
    }

    pub fn save_dialog() -> Self {
        Self {
            purpose: DialogPurpose::Save,
            message: "Do you wanna save this sheet? Please enter a name first.".into(),
            buffer: Some(String::new()),
            background_color: Color::DarkRed,
            answers: DialogAnswers::YesNo,
            selected_answer: 1,
            height: 5,
        }
    }

    pub fn execute_dialog() -> Self {
        Self {
            purpose: DialogPurpose::Execute,
            message: "Execute a command. Type help for a list of all commands.".into(),
            buffer: Some(String::new()),
            background_color: Color::DarkCyan,
            answers: DialogAnswers::Ok,
            selected_answer: 0,
            height: 5,
        }
    }

    pub fn render(&self) -> crossterm::Result<()> {
        let box_height = 5;
        let size = terminal::size()?;
        let width = size.0 as usize;
        execute!(
            stdout(),
            MoveTo(0, (size.1 - box_height) / 2),
            SetBackgroundColor(self.background_color)
        )?;
        for _ in 0..box_height {
            print_blank_line(width);
        }
        let offset = if self.buffer.is_some() { 1 } else { 0 };
        execute!(stdout(), MoveTo(0, (size.1 - box_height) / 2 + 1),)?;
        for line in self.message.lines() {
            execute!(
                stdout(),
                Print(line.pad(width, ' ', pad::Alignment::Middle, true)),
                // MoveDown(1),
            )?;
        }
        execute!(stdout(), MoveDown(1 - offset))?;

        if let Some(buffer) = &self.buffer {
            execute!(
                stdout(),
                MoveToColumn(0),
                Print(
                    buffer
                        .pad(buffer.len().max(32), '_', pad::Alignment::Left, false)
                        .pad(width, ' ', pad::Alignment::Middle, true)
                ),
                MoveDown(1),
            )?;
        }

        match self.answers {
            DialogAnswers::Ok => {
                execute!(
                    stdout(),
                    Print("[Ok]".pad(width, ' ', pad::Alignment::Middle, true))
                )?;
            }
            DialogAnswers::YesNo => match self.selected_answer {
                0 => execute!(
                    stdout(),
                    Print("[Yes]    No".pad(width, ' ', pad::Alignment::Middle, true))
                )?,
                1 => execute!(
                    stdout(),
                    Print("Yes    [No]".pad(width, ' ', pad::Alignment::Middle, true))
                )?,
                _ => unreachable!(),
            },
        }
        Ok(())
    }

    pub(crate) fn update(&mut self, key: KeyEvent) -> crossterm::Result<DialogResult> {
        let mut result = DialogResult::None;
        match key.code {
            KeyCode::Backspace => {
                if let Some(buffer) = &mut self.buffer {
                    buffer.pop();
                }
            }
            KeyCode::Enter => {
                result = match self.answers {
                    DialogAnswers::Ok => DialogResult::Yes(self.buffer.take()),
                    DialogAnswers::YesNo => match self.selected_answer {
                        0 => DialogResult::Yes(self.buffer.take()),
                        1 => DialogResult::Close,
                        _ => unreachable!(),
                    },
                }
            }
            KeyCode::Left => {
                if self.selected_answer > 0 {
                    self.selected_answer -= 1;
                }
            }
            KeyCode::Right => {
                if self.selected_answer + 1 < self.answers.len() {
                    self.selected_answer += 1;
                }
            }
            KeyCode::Up => todo!(),
            KeyCode::Down => todo!(),
            KeyCode::Home => todo!(),
            KeyCode::End => todo!(),
            KeyCode::PageUp => todo!(),
            KeyCode::PageDown => todo!(),
            KeyCode::Tab => todo!(),
            KeyCode::BackTab => todo!(),
            KeyCode::Delete => todo!(),
            KeyCode::Insert => todo!(),
            KeyCode::F(_) => todo!(),
            KeyCode::Char(ch) => {
                if let Some(buffer) = &mut self.buffer {
                    buffer.push(ch);
                }
            }
            KeyCode::Null => todo!(),
            KeyCode::Esc => {
                result = DialogResult::Close;
            }
            KeyCode::CapsLock => todo!(),
            KeyCode::ScrollLock => todo!(),
            KeyCode::NumLock => todo!(),
            KeyCode::PrintScreen => todo!(),
            KeyCode::Pause => todo!(),
            KeyCode::Menu => todo!(),
            KeyCode::KeypadBegin => todo!(),
            KeyCode::Media(_) => todo!(),
            KeyCode::Modifier(_) => todo!(),
        }
        Ok(result)
    }

    pub(crate) fn clear(box_height: usize) -> crossterm::Result<()> {
        let size = terminal::size()?;
        execute!(
            stdout(),
            MoveTo(0, (size.1 - box_height as u16) / 2),
            ResetColor,
        )?;
        for _ in 0..box_height {
            print_blank_line(size.0 as _);
        }
        Ok(())
    }

    pub(crate) fn purpose(&self) -> DialogPurpose {
        self.purpose
    }
}

pub enum DialogResult {
    None,
    Close,
    Yes(Option<String>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DialogAnswers {
    Ok,
    YesNo,
}

impl DialogAnswers {
    pub fn len(&self) -> usize {
        match self {
            DialogAnswers::Ok => 1,
            DialogAnswers::YesNo => 2,
        }
    }
}
