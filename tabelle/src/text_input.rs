use unicode_width::UnicodeWidthStr;

#[derive(Debug, Default)]
pub struct TextInput {
    pub buffer: String,
    byte_cursor: usize,
    char_cursor: usize,
}

impl TextInput {
    pub fn insert_char(&mut self, ch: char) {
        self.buffer.insert(self.byte_cursor, ch);
        self.byte_cursor += ch.len_utf8();
        self.char_cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.byte_cursor == 0 {
            return;
        }
        self.byte_cursor = (self.byte_cursor.saturating_sub(5)..self.byte_cursor - 1)
            .filter(|&i| self.buffer.is_char_boundary(i))
            .rev()
            .next()
            .unwrap();
        self.buffer.remove(self.byte_cursor);
        self.char_cursor -= 1;
    }

    pub fn delete(&mut self) {
        if self.byte_cursor == self.buffer.len() {
            return;
        }
        self.buffer.remove(self.byte_cursor);
    }

    pub fn left(&mut self) {
        self.byte_cursor = (self.byte_cursor.saturating_sub(5)..self.byte_cursor - 1)
            .filter(|&i| self.buffer.is_char_boundary(i))
            .rev()
            .next()
            .unwrap();
        self.char_cursor = self.char_cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        if self.byte_cursor != self.buffer.len() {
            self.byte_cursor = (self.byte_cursor + 1..self.byte_cursor + 5)
                .filter(|&i| self.buffer.is_char_boundary(i))
                .next()
                .unwrap();
            self.char_cursor += 1;
        }
    }

    pub fn up(&mut self) {
        self.byte_cursor = 0;
        self.char_cursor = 0;
    }

    pub fn down(&mut self) {
        self.byte_cursor = self.buffer.len();
        self.char_cursor = self.buffer.width();
    }

    pub fn clear(&mut self) {
        self.byte_cursor = 0;
        self.char_cursor = 0;
        self.buffer.clear();
    }

    pub fn set(&mut self, arg: &str) {
        self.buffer = arg.into();
        self.byte_cursor = self.buffer.len();
        self.char_cursor = self.buffer.width();
    }

    pub(crate) fn cursor(&self) -> usize {
        self.char_cursor
    }

    pub(crate) fn set_cursor(&mut self, cursor: usize) {
        self.char_cursor = cursor;
        self.byte_cursor = self.buffer.char_indices().nth(cursor).unwrap().0;
    }
}
