
use std::fmt;

#[derive(Clone, Debug)]
pub enum ErrorKind {
    FileError{ msg: String },
    EmacsCallFailed,
    PdfLatexCallFailed,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::FileError{msg} => write!(f, "{}", msg),
            ErrorKind::EmacsCallFailed => write!(f, "calling Emacs failed"),
            ErrorKind::PdfLatexCallFailed => write!(f, "calling pdflatex failed"),
        }
    }
}
