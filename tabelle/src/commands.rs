use std::{fmt::Display, io::stdout, path::PathBuf};

use crossterm::{
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use strum::{Display, EnumVariantNames};
use tabelle_core::units::UnitKind;

#[derive(strum::EnumIter, Display, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub enum CommandKind {
    None,
    Help,
    New,
    Set,
    Save,
    Find,
    Sort,
    Fit,
    Fix,
    Resize,
    Clear,
    Fill,
    Goto,
}

impl CommandKind {
    pub fn description(&self) -> &'static str {
        match self {
            CommandKind::None => "",
            CommandKind::Help => "Displays this help with an overview over all commands and a general tutorial for this application.",
            CommandKind::New => "Creates a new spreadsheet. Make sure to save before.",
            CommandKind::Set => "Change the current cell. Takes two arguments, the first is the property, which will be changed (see the example for all possible values) and the second is the value for that key.",
            CommandKind::Save => "Saves the current spreadsheet to a path.",
            CommandKind::Find => "Finds a string in all the cells. Starts looking at the current cell, so you can checkout all results by repeating the command.",
            CommandKind::Sort => "Takes a column (case insensitive) as an argument. This sorts the spreadsheet by this column. The ordering is `Text > Numbers > Empty`, where text is sorted alphabetically and numbers by their value. Formulas are ordered by their last evaluated value (which is the one displayed).",
            CommandKind::Fit => "Sets the width of the given column automatically, so that its content fits inside.",
            CommandKind::Fix => "This pins the given number of rows to the top. They will not be sorted.",
            CommandKind::Resize => "Takes the new number of columns and rows as arguments. The have to be >= then the old size, otherwise bugs might be triggered.",
            CommandKind::Clear => "Clears the cells between the current cell and the supplied cell of any content.",
            CommandKind::Fill => "Auto fills from the current cell to the given cell.",
            CommandKind::Goto => "Go to a given cell. Can also be accessed by pressing Ctrl+G.",
        }
    }

    pub fn example_values(&self) -> Vec<Command> {
        match self {
            CommandKind::None => vec![Command::None],
            CommandKind::Help => vec![Command::Help],
            CommandKind::New => vec![Command::New],
            CommandKind::Set => vec![
                Command::Set(SetCommand::ColumnWidth(10)),
                Command::Set(SetCommand::Unit(UnitKind::Dollar)),
            ],
            CommandKind::Save => vec![Command::Save("table.xlsx".into())],
            CommandKind::Find => vec![Command::Find("total".into())],
            CommandKind::Sort => vec![Command::Sort(0)],
            CommandKind::Fit => vec![Command::Fit(0)],
            CommandKind::Fix => vec![Command::Fix(1), Command::Fix(5)],
            CommandKind::Resize => vec![Command::Resize(5, 5)],
            CommandKind::Clear => vec![Command::Clear((3, 2))],
            CommandKind::Fill => vec![Command::Fill((5, 5))],
            CommandKind::Goto => vec![Command::Goto((0, 550))],
        }
    }
}

impl From<Command> for CommandKind {
    fn from(value: Command) -> Self {
        match value {
            Command::None => Self::None,
            Command::Help => Self::Help,
            Command::New => Self::New,
            Command::Set(_) => Self::Set,
            Command::Save(_) => Self::Save,
            Command::Find(_) => Self::Find,
            Command::Sort(_) => Self::Sort,
            Command::Fit(_) => Self::Fit,
            Command::Fix(_) => Self::Fix,
            Command::Resize(_, _) => Self::Resize,
            Command::Clear(_) => Self::Clear,
            Command::Fill(_) => Self::Fill,
            Command::Goto(_) => Self::Goto,
        }
    }
}

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
                    ["save", path] => {
                        Ok(Self::Save(std::path::PathBuf::from(path.to_owned()).into()))
                    }
                    ["find", needle] => Ok(Self::Find(needle.to_string().into())),
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
                    terminal
                        .spreadsheet
                        .cell_at_mut(terminal.spreadsheet.current_cell())
                        .set_unit(*unit);
                    true
                }
            },
            Command::Save(path) => {
                terminal.spreadsheet.save_as_xlsx(path);
                true
            }
            Command::Find(needle) => {
                if let Some(cell_position) = terminal.spreadsheet.find(needle) {
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
                let cell = (
                    cell.0.min(terminal.spreadsheet.columns() - 1),
                    cell.1.min(terminal.spreadsheet.rows() - 1),
                );
                terminal.set_cursor(cell.0, cell.1)?;
                true
            }
        };
        Ok(exits_command_mode)
    }
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
