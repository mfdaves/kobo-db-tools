pub mod bookmark;
pub mod error;

use error::ExportError;

pub enum ExportFormat {
    Csv,
    Markdown,
}

pub trait Export {
    fn to_csv(&self) -> Result<String, ExportError>;
    fn to_md(&self) -> Result<String, ExportError>;
    fn to_json(&self) -> Result<String, ExportError>;
}
