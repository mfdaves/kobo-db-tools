use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExportError {
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("CSV into inner error: {0}")]
    CsvIntoInner(#[from] csv::IntoInnerError<csv::Writer<Vec<u8>>>),
    #[error("serde_json::to_string error: {0}")]
    JsonToString(#[from] serde_json::Error),
}
