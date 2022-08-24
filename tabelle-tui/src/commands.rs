use strum::{Display, EnumVariantNames};

#[derive(Debug, EnumVariantNames, Display)]
pub enum Command {
    Help,
    Unknown(String),
}

impl Command {
    pub fn parse(command: String) -> Self {
        match command.as_str() {
            "help" => Self::Help,
            _ => Self::Unknown(command),
        }
    }
}
