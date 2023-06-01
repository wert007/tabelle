use std::{cmp::Ordering, str::FromStr};

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum CsvParseError {
    NoSuccessfullParse(Box<CsvParseError>),
    InvalidEscaping,
    NoCellsFound(usize, usize),
    UnfinishedEscaping,
}

impl FromStr for CsvFile {
    type Err = CsvParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut size: Vec<Result<(char, (usize, usize)), _>> = KNOWN_SEPERATORS
            .chars()
            .map(|sep| parse_size_of_csv(s, sep).map(|s| (sep, s)))
            .collect();
        size.sort_unstable_by(|a, b| match (a, b) {
            (Ok(a), Ok(b)) => b.1.cmp(&a.1),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => Ordering::Equal,
        });
        let ((width, height), seperator) = if let Ok((seperator, size)) = size[0] {
            (size, seperator)
        } else {
            // We know that the first one is an error, since otherwise we would
            // have an valid index found.
            return Err(CsvParseError::NoSuccessfullParse(Box::new(
                size[0].as_ref().unwrap_err().clone(),
            )));
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
        let mut current_width = !line.is_empty() as _;
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
            match state {
                CsvParseState::NewCell => {}
                CsvParseState::InCell => {}
                CsvParseState::InCellEndEscape => {}
                CsvParseState::InCellEscaped => return Err(CsvParseError::UnfinishedEscaping),
            }
        }
        if width < current_width {
            width = current_width;
        }
        height += (current_width > 0) as usize;
    }
    if width == 0 || height == 0 {
        Err(CsvParseError::NoCellsFound(width, height))
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
                default => {
                    current_cell.push(default);
                    match state {
                        CsvParseState::NewCell => state = CsvParseState::InCell,
                        CsvParseState::InCell | CsvParseState::InCellEscaped => {}
                        CsvParseState::InCellEndEscape => {
                            return Err(CsvParseError::InvalidEscaping)
                        }
                    }
                }
            }
        }
        cells.push(current_cell);
        current_cell = String::with_capacity(capacity);
        while cells.len() < cell_count + width {
            cells.push(String::new());
        }
    }

    assert_eq!(cells.len(), width * height);
    Ok(CsvFile {
        cells,
        width,
        height,
        seperator,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn regression_test1() {
        let fail_csv = "Filename;Stmts;Miss;Cover;Missing
src/application.rs;29;29;0.00%;87-137
src/pattern/buffer.rs;21;0;100.00%;
src/pattern/io_processor.rs;19;19;0.00%;72-160
src/pattern/reader.rs;31;20;35.48%;57-58, 75-79, 90-109, 123-139
src/changelog/comment_changes.rs;163;163;0.00%;106-399";

        let mut size: Vec<Result<(char, (usize, usize)), _>> = KNOWN_SEPERATORS
            .chars()
            .map(|sep| parse_size_of_csv(fail_csv, sep).map(|s| (sep, s)))
            .collect();

        for (actual, expected) in
            size.iter()
                .zip([(',', (4usize, 6usize)), (';', (5, 6)), ('\t', (1, 6))])
        {
            let actual = *actual.as_ref().expect("Should not fail!");
            assert_eq!(actual, expected);
        }

        size.sort_unstable_by(|a, b| match (a, b) {
            (Ok(a), Ok(b)) => b.1.cmp(&a.1),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => Ordering::Equal,
        });

        let expected = [(';', (5, 6)), (',', (4, 6)), ('\t', (1, 6))];

        for (actual, expected) in size.iter().zip(expected) {
            let actual = *actual.as_ref().expect("Should not fail!");
            assert_eq!(actual, expected);
        }

        let csv = match &size[0] {
            Ok(it) => parse_csv(fail_csv, it.0, it.1 .0, it.1 .1),
            Err(err) => panic!("{err:?}"),
        }
        .unwrap();

        dbg!(&csv);
        assert_eq!(csv.width, 5);
        assert_eq!(csv.height, 6);
        assert_eq!(csv.seperator, ';');
        assert_eq!(
            csv.cells,
            [
                "Filename",
                "Stmts",
                "Miss",
                "Cover",
                "Missing",
                "src/application.rs",
                "29",
                "29",
                "0.00%",
                "87-137",
                "src/pattern/buffer.rs",
                "21",
                "0",
                "100.00%",
                "",
                "src/pattern/io_processor.rs",
                "19",
                "19",
                "0.00%",
                "72-160",
                "src/pattern/reader.rs",
                "31",
                "20",
                "35.48%",
                "57-58, 75-79, 90-109, 123-139",
                "src/changelog/comment_changes.rs",
                "163",
                "163",
                "0.00%",
                "106-399",
            ],
        );
    }
}
