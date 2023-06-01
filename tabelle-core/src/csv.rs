use std::str::FromStr;

pub struct CsvFile {
    // NOTE: It might be possible to use Cow<str> here, but it seems to be
    // premature to use it right now, since lifetimes tend to complicate things,
    // when you are not sure yet how something is used.
    pub cells: Vec<String>,
    pub width: usize,
    pub height: usize,
    pub seperator: char,
}

const KNOWN_SEPERATORS: &str = ",;\t";

#[derive(Debug)]
pub enum CsvParseError {
    NoSuccessfullParse,
    InvalidEscaping,
    NoCellsFound,
}

impl FromStr for CsvFile {
    type Err = CsvParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let size: Vec<Result<(usize, usize), _>> = KNOWN_SEPERATORS
            .chars()
            .map(|sep| parse_size_of_csv(s, sep))
            .collect();
        let ((width, height), seperator) = if let Some(index) = size.iter().position(|s| s.is_ok())
        {
            (
                *size[index].as_ref().unwrap(),
                KNOWN_SEPERATORS.chars().nth(index).expect(
                    "size comes from KNOWN_SEPERATORS, so they should have the same length",
                ),
            )
        } else {
            // We know that the first one is an error, since otherwise we would
            // have an valid index found.
            return Err(CsvParseError::NoSuccessfullParse);
        };
        parse_csv(s, seperator, width, height)
    }
}

#[derive(Debug, PartialEq)]
enum CsvParseState {
    NewCell,
    InCell,
    InCellEscaped,
    InCellEndEscape,
}

fn parse_size_of_csv(s: &str, sep: char) -> Result<(usize, usize), CsvParseError> {
    let mut width = 0;
    let mut height = 0;

    let mut state = CsvParseState::NewCell;

    for line in s.lines() {
        let mut current_width = line.is_empty() as _;
        for ch in line.chars() {
            match ch {
                '"' if state != CsvParseState::InCell => {
                    // This handles starting and ending quotes and also double
                    // quotes in the middle of the escaped text. Normal quotes
                    // in a cell are not handled here. If a escaped text is not
                    // ended at the end this functions returns a
                    // CsvParseError::InvalidEscaping in the next iteration
                    // of this loop.
                    state = match state {
                        CsvParseState::InCellEndEscape | CsvParseState::NewCell => {
                            CsvParseState::InCellEscaped
                        }
                        CsvParseState::InCell => {
                            unreachable!("The if guard should make this impossible!")
                        }
                        CsvParseState::InCellEscaped => CsvParseState::InCellEndEscape,
                    };
                }
                seperator
                    if seperator == sep
                        && matches!(
                            state,
                            CsvParseState::InCell | CsvParseState::InCellEndEscape
                        ) =>
                {
                    current_width += 1;
                    state = CsvParseState::NewCell;
                }
                _ => match state {
                    CsvParseState::NewCell => state = CsvParseState::InCell,
                    CsvParseState::InCell | CsvParseState::InCellEscaped => {}
                    CsvParseState::InCellEndEscape => return Err(CsvParseError::InvalidEscaping),
                },
            }
        }
        if width < current_width {
            width = current_width;
        }
        height += line.is_empty() as usize;
    }
    if width == 0 || height == 0 {
        Err(CsvParseError::NoCellsFound)
    } else {
        Ok((width, height))
    }
}

fn parse_csv(
    s: &str,
    seperator: char,
    width: usize,
    height: usize,
) -> Result<CsvFile, CsvParseError> {
    let mut cells = Vec::with_capacity(width * height);
    let capacity = s.len() / (width * height);
    let mut current_cell = String::with_capacity(capacity);
    let mut state = CsvParseState::NewCell;

    for line in s.lines() {
        let cell_count = cells.len();
        for ch in line.chars() {
            match ch {
                '"' if state != CsvParseState::InCell => {
                    // This handles starting and ending quotes and also double
                    // quotes in the middle of the escaped text. Normal quotes
                    // in a cell are not handled here. If a escaped text is not
                    // ended at the end this functions returns a
                    // CsvParseError::InvalidEscaping in the next iteration
                    // of this loop.
                    state = match state {
                        CsvParseState::InCellEndEscape => {
                            current_cell.push('"');
                            CsvParseState::InCellEscaped
                        }
                        CsvParseState::NewCell => CsvParseState::InCellEscaped,
                        CsvParseState::InCell => {
                            unreachable!("The if guard should make this impossible!")
                        }
                        CsvParseState::InCellEscaped => CsvParseState::InCellEndEscape,
                    };
                }
                sep if seperator == sep
                    && matches!(
                        state,
                        CsvParseState::InCell | CsvParseState::InCellEndEscape
                    ) =>
                {
                    cells.push(current_cell);
                    current_cell = String::with_capacity(capacity);
                    state = CsvParseState::NewCell;
                }
                default => match state {
                    CsvParseState::NewCell => state = CsvParseState::InCell,
                    CsvParseState::InCell | CsvParseState::InCellEscaped => {
                        current_cell.push(default)
                    }
                    CsvParseState::InCellEndEscape => return Err(CsvParseError::InvalidEscaping),
                },
            }
        }
        while cells.len() < cell_count + width {
            cells.push(String::new());
        }
    }

    Ok(CsvFile {
        cells,
        width,
        height,
        seperator,
    })
}
