
#[derive(Copy, Clone, Debug)]
pub enum ErrorKind {
    FileOpenError,
    FileReadError,
    FileCreationError,
    FileWriteError,
    EmacsCallFailed,
    PdfLatexCallFailed,
    InvalidOutputFormat,
}
