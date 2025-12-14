/// solana client mode
#[derive(Debug, Clone, Copy)]
pub enum Mode {
    MAIN,
    TEST,
    DEV,
}

/// unified result
pub type UnifiedResult<T, E> = Result<T, UnifiedError<E>>;

/// unified error
#[derive(Debug)]
pub enum UnifiedError<T> {
    Error(T),
}

#[derive(Debug, Clone, Copy)]

pub enum Direction {
    In,
    Out,
}
