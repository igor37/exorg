
#[derive(Clone, Debug)]
pub enum ErrorKind {
    FileError{ msg: String },
    EmacsCallFailed,
    PdfLatexCallFailed,
    InvalidOutputFormat,
}
