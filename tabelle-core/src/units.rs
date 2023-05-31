use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitKind {
    #[default]
    None,
    Dollar,
}

impl Display for UnitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UnitKind::None => "",
                UnitKind::Dollar => "$",
            }
        )
    }
}

impl UnitKind {
    pub(crate) fn display<'a>(&self, content: &'a crate::CellContent) -> std::borrow::Cow<'a, str> {
        match content {
            crate::CellContent::Empty => "".into(),
            crate::CellContent::Text(it) => it.into(),
            &crate::CellContent::Number(it) => match self {
                UnitKind::None => it.to_string(),
                UnitKind::Dollar => format!("$ {:.2}", it as f64 * 0.01),
            }
            .into(),
            crate::CellContent::FloatNumber(it, _) => it.to_string().into(),
            crate::CellContent::Formula(it) => match it.value() {
                crate::cells::cell_content::Value::String(it) => it.into(),
                &crate::cells::cell_content::Value::Number(it) => match self {
                    UnitKind::None => it.to_string(),
                    UnitKind::Dollar => format!("$ {:.2}", it as f64 * 0.01),
                }.into(),
                crate::cells::cell_content::Value::FloatNumber(it) => it.to_string().into(),
                crate::cells::cell_content::Value::Empty => "".into(),
                crate::cells::cell_content::Value::Error => "#error".into(),
            },
        }
    }
}

impl<'a> TryFrom<&'a umya_spreadsheet::NumberingFormat> for UnitKind {
    type Error = &'a umya_spreadsheet::NumberingFormat;

    fn try_from(value: &'a umya_spreadsheet::NumberingFormat) -> Result<Self, Self::Error> {
        match value.get_format_code() {
            umya_spreadsheet::NumberingFormat::FORMAT_CURRENCY_USD => {
                Ok(Self::Dollar)
            }
            umya_spreadsheet::NumberingFormat::FORMAT_GENERAL => {
                Ok(Self::None)
            }
            _ => Err(value)
        }
    }
}

impl Into<umya_spreadsheet::NumberingFormat> for UnitKind {
    fn into(self) -> umya_spreadsheet::NumberingFormat {
        let format = match self {
            UnitKind::None => umya_spreadsheet::NumberingFormat::FORMAT_GENERAL,
            UnitKind::Dollar => umya_spreadsheet::NumberingFormat::FORMAT_CURRENCY_USD,
        };
        let mut nf = umya_spreadsheet::NumberingFormat::default();
        nf.set_format_code(format);
        nf
    }
}