/// solana client mode
#[derive(Debug, Clone, Copy)]
pub enum Mode {
    MAIN,
    TEST,
    DEV,
}

/// unified result
pub type UnifiedResult<T> = Result<T, UnifiedError>;

/// unified error
#[derive(Debug)]
pub enum UnifiedError {
    Error(String),
}
