pub mod bookmark;
pub mod dictionary;
pub mod error;
pub mod sessions;

use error::ExportError;

pub trait Export {
    fn to_csv(&self) -> Result<String, ExportError>;
    fn to_md(&self) -> Result<String, ExportError>;
    fn to_json(&self) -> Result<String, ExportError>;
}
