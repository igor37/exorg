
#[derive(Copy, Clone, Debug)]
pub enum ErrorKind {
    Unimplemented,
    FileOpenError,
    FileReadError,
    FileCreationError,
    FileWriteError,
    EmacsCallFailed,
}
