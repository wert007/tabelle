use std::{fmt::Display, io::stdout, path::PathBuf};

use crossterm::{
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use strum::{Display, EnumVariantNames};
use tabelle_core::units::UnitKind;

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
    Clear((usize, usize)),
    Fill((usize, usize)),
    Goto((usize, usize)),
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
                    ["clear", cell] => Ok(Self::Clear(tabelle_core::cell_name_to_position(cell)?)),
                    ["fill", cell] => Ok(Self::Fill(tabelle_core::cell_name_to_position(cell)?)),
                    ["goto", cell] => Ok(Self::Goto(tabelle_core::cell_name_to_position(cell)?)),
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
            Command::Goto(cell) | Command::Clear(cell) | Command::Fill(cell) => {
                format!("{self} {}", tabelle_core::cell_position_to_name(*cell))
            }
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
                SetCommand::Unit(unit) => {
                    terminal.spreadsheet.cell_at_mut(terminal.spreadsheet.current_cell()).set_unit(*unit);
                    true
                }
            },
            Command::Save(path) => {
                terminal.spreadsheet.save_as_xlsx(path);
                true
            }
            Command::Find(needle) => {
                if let Some(cell_position) = terminal.spreadsheet.find(&needle) {
                    let old_cursor = terminal.scroll_page.cursor;
                    terminal.spreadsheet.set_cursor(cell_position);
                    terminal
                        .scroll_page
                        .set_cursor(cell_position, terminal.cell_size());
                    terminal.update_cursor(old_cursor)?;
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
            &Command::Clear((to_x, to_y)) => {
                let (from_x, from_y) = terminal.spreadsheet.current_cell();
                for x in from_x..=to_x {
                    for y in from_y..=to_y {
                        terminal
                            .spreadsheet
                            .update_cell_at((x, y), tabelle_core::CellContent::Empty);
                        if !terminal.move_cursor(0, 1)? {
                            break;
                        }
                    }
                    // TODO: Fix handling, if the break before was triggered,
                    // since then we did not move to_y - from_y cells.
                    if !terminal.move_cursor(1, -((to_y - from_y) as isize))? {
                        break;
                    }
                }
                terminal.spreadsheet.evaluate();
                terminal.update_cursor((from_x, from_y))?;
                true
            }
            &Command::Fill((to_x, to_y)) => {
                let (from_x, from_y) = terminal.spreadsheet.current_cell();
                for x in from_x..=to_x {
                    for y in from_y..=to_y {
                        terminal.spreadsheet.update_cell_at(
                            (x, y),
                            terminal
                                .spreadsheet
                                .recommended_cell_content((from_x, from_y)),
                        );
                        terminal.spreadsheet.evaluate();
                        if !terminal.move_cursor(0, 1)? {
                            break;
                        }
                    }
                    assert!(
                        to_y >= from_y && to_x >= from_x,
                        "to_y = {to_y}; from_y = {from_y}; to_x = {to_x}; from_x = {from_x}; x = {x}"
                    );
                    // TODO: Fix handling, if the break before was triggered,
                    // since then we did not move to_y - from_y cells.
                    if !terminal.move_cursor(1, -((to_y - from_y) as isize))? {
                        break;
                    }
                }
                terminal.update_cursor((from_x, from_y))?;
                true
            }
            &Command::Goto(cell) => {
                terminal.set_cursor(cell.0, cell.1)?;
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
    Clear(_cell), Clear((0, 3)), "Clears the cells between the current cell and the supplied cell of any content.";
    Fill(_cell), Fill((0, 3)), "Autofills the cells between the current cell and the supplied cell.";
    Goto(_cell), Goto((0, 3)), "Moves to the entered cell. Can also be accessed by pressing Ctrl+G.";
}

fn parse_set_command<'a>(key: &'a str, value: &'a str) -> Result<Command, &'a str> {
    Ok(match key {
        "column-width" => {
            let value: usize = value.parse().map_err(|_| "column-width expected integer")?;
            Command::Set(SetCommand::ColumnWidth(value))
        }
        "unit" => {
            let value = match value {
                "$" => UnitKind::Dollar,
                _ => return Err("Invalid unit kind found"),
            };
            Command::Set(SetCommand::Unit(value))
        }
        _ => return Err(key),
    })
}

#[derive(Debug, EnumVariantNames, PartialEq)]
pub enum SetCommand {
    ColumnWidth(usize),
    Unit(UnitKind),
}

impl Display for SetCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetCommand::ColumnWidth(width) => write!(f, "column-width {width}"),
            SetCommand::Unit(unit) => write!(f, "unit {unit}"),
        }
    }
}
