
use std::fmt;

#[derive(Clone, Debug)]
pub enum ErrorKind {
    FileError{ msg: String },
    EmacsCallFailed,
    PandocCallFailed,
    PdfLatexCallFailed,
    CodeBlockNotFound,
    AmbiguousCodeBlockName,
    UnsatisfiableDependencies,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::FileError{msg}             => write!(f, "{}", msg),
            ErrorKind::EmacsCallFailed            => write!(f, "calling Emacs failed"),
            ErrorKind::PandocCallFailed           => write!(f, "calling Pandoc failed"),
            ErrorKind::PdfLatexCallFailed         => write!(f, "calling pdflatex failed"),
            ErrorKind::CodeBlockNotFound          => write!(f, "specified code block not found"),
            ErrorKind::AmbiguousCodeBlockName     => write!(f, "muliple code blocks match given name"),
            ErrorKind::UnsatisfiableDependencies  => write!(f, "dependencies can't be satisfied"),
        }
    }
}
