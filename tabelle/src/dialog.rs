use std::{fmt::Debug, io::stdout};

use crossterm::{
    cursor::{MoveDown, MoveTo, MoveToColumn},
    event::{KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor},
    terminal,
};
use serde::{Deserialize, Serialize};
use unicode_truncate::UnicodeTruncateStr;

use crate::print_blank_line;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DialogPurpose {
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
                Print(line.unicode_pad(width, unicode_truncate::Alignment::Center, true)),
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
                        // .unicode_pad(buffer.len().max(32), '_', unicode_truncate::Alignment::Left, false)
                        .unicode_pad(width, unicode_truncate::Alignment::Center, true)
                ),
                MoveDown(1),
            )?;
        }

        match self.answers {
            DialogAnswers::Ok => {
                execute!(
                    stdout(),
                    Print("[Ok]".unicode_pad(width, unicode_truncate::Alignment::Center, true))
                )?;
            }
            DialogAnswers::YesNo => match self.selected_answer {
                0 => execute!(
                    stdout(),
                    Print("[Yes]    No".unicode_pad(
                        width,
                        unicode_truncate::Alignment::Center,
                        true
                    ))
                )?,
                1 => execute!(
                    stdout(),
                    Print("Yes    [No]".unicode_pad(
                        width,
                        unicode_truncate::Alignment::Center,
                        true
                    ))
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
