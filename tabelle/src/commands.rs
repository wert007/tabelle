use std::{fmt::Display, io::stdout, path::PathBuf};

use crossterm::{
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use strum::{Display, EnumVariantNames};

#[derive(Debug, EnumVariantNames, Display, strum::EnumDiscriminants, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub enum Command {
    None,
    Help,
    New,
    Set(SetCommand),
    Save(PathBuf),
    Find(String),
    Sort(usize),
    Fit(usize),
    Fix(usize),
    Resize(usize, usize),
}

impl Command {
    pub fn parse(text: &str) -> Result<Self, &str> {
        match text {
            "" => Ok(Self::None),
            "help" => Ok(Self::Help),
            "new" => Ok(Self::New),
            err => {
                let parts: Vec<&str> = text.split(' ').collect();
                match &parts[..] {
                    ["set", key, value] => parse_set_command(key, value),
                    ["save", path] => Ok(Self::Save(std::path::PathBuf::from(path.to_owned()))),
                    ["find", needle] => Ok(Self::Find(needle.to_string())),
                    ["sort", column] => Ok(Self::Sort(
                        tabelle_core::column_name_to_index(&column.to_ascii_uppercase())
                            .map_err(|_| *column)?,
                    )),
                    ["fit", column] => Ok(Self::Fit(
                        tabelle_core::column_name_to_index(&column.to_ascii_uppercase())
                            .map_err(|_| *column)?,
                    )),
                    ["fix", row, "rows"] => Ok(Self::Fix(row.parse().map_err(|_| *row)?)),
                    ["fix", "1", "row"] => Ok(Self::Fix(1)),
                    ["resize", width, height] => Ok(Self::Resize(
                        width.parse().map_err(|_| *width)?,
                        height.parse().map_err(|_| *height)?,
                    )),
                    _ => Err(err),
                }
            }
        }
    }

    pub fn full_display(&self) -> String {
        match self {
            Command::Set(kind) => format!("{self} {kind}"),
            Command::Save(path) => format!("{self} {}", path.display()),
            Command::Find(text) => format!("{self} {text}"),
            Command::Sort(column) => format!("{self} {}", tabelle_core::to_column_name(*column)),
            Command::Fit(column) => format!("{self} {}", tabelle_core::to_column_name(*column)),
            Command::Fix(rows) => {
                format!("{self} {rows} {}", if *rows == 1 { "row" } else { "rows" })
            }
            Command::Resize(columns, rows) => format!("{self} {columns} {rows}"),
            default => default.to_string(),
        }
    }

    pub(crate) fn execute(&self, terminal: &mut crate::Terminal) -> crossterm::Result<bool> {
        let exits_command_mode = match self {
            Command::None => true,
            Command::Help => {
                terminal.render_help()?;
                false
            }
            Command::New => {
                terminal.set_cursor(0, 0)?;
                terminal.spreadsheet = tabelle_core::Spreadsheet::new(5, 5);
                stdout().execute(Clear(ClearType::All))?;
                true
            }
            Command::Set(command) => match command {
                SetCommand::ColumnWidth(width) => {
                    let column = terminal.spreadsheet.current_cell().0;
                    terminal.spreadsheet.set_column_width(column, *width);
                    true
                }
            },
            Command::Save(path) => {
                terminal.spreadsheet.save_as_xlsx(path);
                true
            }
            Command::Find(needle) => {
                if let Some(cell_position) = terminal.spreadsheet.find(&needle) {
                    terminal.spreadsheet.set_cursor(cell_position);
                    terminal
                        .scroll_page
                        .set_cursor(cell_position, terminal.cell_size());
                    terminal.update_cursor()?;
                }
                true
            }
            &Command::Sort(column) => {
                terminal.spreadsheet.sort_column(column);
                // terminal.render()?;
                true
            }
            &Command::Fit(column) => {
                terminal.spreadsheet.fit_column_width(column);
                // terminal.render()?;
                true
            }
            &Command::Fix(rows) => {
                terminal.spreadsheet.fix_rows(rows);
                true
            }
            &Command::Resize(width, height) => {
                terminal.spreadsheet.resize(width, height);
                true
            }
        };
        Ok(exits_command_mode)
    }
}

macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

macro_rules! command_helper {
    ($($command:pat, $default_value:expr, $help:literal;)*) => {
        impl Command {
            pub fn command_values() -> [Command; count!($($command)*) ] {
                use Command::*;
                [
                    $($default_value,)*
                ]
            }

            pub fn help(&self) -> &'static str {
                use Command::*;
                match self {
                    $($command => {
                        $help
                    })*
                }
            }
        }
    };
}

command_helper! {
    None, None, "";
    Help, Help, "Displays this help with an overview over all commands and a general tutorial for this application.";
    New, New, "Erases the current table completely. Make sure to save beforehand. The program won't remind you of it! Also: Creates a new table.";
    Set(_kind), Set(SetCommand::ColumnWidth(10)), "Takes two arguments, the first is the key, which may be one of [column-width] and the second is the value for that key. TODO: Add an explanation for all keys as a seperate section to this help.";
    Save(_path), Save(PathBuf::from(String::from("table.xlsx"))), "Saves the current spreadsheet as an .xlsx file, this can also be accessed by pressing CTRL+S. Takes the path to the spreadsheet as an argument.";
    Find(_text), Find(String::from("mouse")), "Finds a string in all the cells. Starts looking at the current cell, so you can checkout all results by repeating the command.";
    Sort(_column), Sort(0), "Takes a column (case insensitive) as an argument. This sorts the spreadsheet by this column. The ordering is `Text > Numbers > Empty`, where text is sorted alphabetically and numbers by their value. Formulas are ordered by their last evaluated value (which is the one displayed.)";
    Fit(_column), Fit(0), "Sets the width of the given column automatically, so that its content fits inside.";
    Fix(_row_count), Fix(1), "Called as `fix 1 row` or `fix 5 rows`. This pins the given number of rows to the top. They will not be sorted. TODO: They should also not be scrolled away.";
    Resize(_columns, _rows), Resize(5, 5), "Takes the new number of columns and rows as arguments. The have to be >= then the old size, otherwise bugs might be triggered.";
}

fn parse_set_command<'a>(key: &'a str, value: &'a str) -> Result<Command, &'a str> {
    Ok(match key {
        "column-width" => {
            let value: usize = value.parse().map_err(|_| "column-width expected integer")?;
            Command::Set(SetCommand::ColumnWidth(value))
        }
        _ => return Err(key),
    })
}

#[derive(Debug, EnumVariantNames, PartialEq)]
pub enum SetCommand {
    ColumnWidth(usize),
}

impl Display for SetCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetCommand::ColumnWidth(width) => write!(f, "column-width {width}"),
        }
    }
}
