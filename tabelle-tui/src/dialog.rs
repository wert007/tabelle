use std::io::stdout;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dialog {
    pub message: String,
    pub buffer: Option<String>,
    pub background_color: Color,
    pub answers: DialogAnswers,
    pub selected_answer: usize,
}

impl Dialog {
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
        execute!(
            stdout(),
            MoveTo(0, size.1 / 2 - 1 - offset),
            Print(self.message.pad(width, ' ', pad::Alignment::Middle, true)),
            MoveDown(2 - offset),
        )?;

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
                    DialogAnswers::Ok => DialogResult::Close,
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
            KeyCode::Esc => todo!(),
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

    pub(crate) fn clear() -> crossterm::Result<()> {
        let box_height = 5;
        let size = terminal::size()?;
        execute!(stdout(), MoveTo(0, (size.1 - box_height) / 2), ResetColor,)?;
        for _ in 0..box_height {
            print_blank_line(size.0 as _);
        }
        Ok(())
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
