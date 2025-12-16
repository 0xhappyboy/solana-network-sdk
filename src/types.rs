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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DexProgramType {
    PumpBondCurve,
    PumpAAM,
    Raydium,
    Meteora,
    Orca,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DexPoolType {
    PumpBondCurve,
    PumpAAM,
    Raydium,
    Meteora,
    Orca,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionType {
    Swap,
    AddLiquidity,
    RemoveLiquidity,
    Binary,
    Other,
    Raw,
    TokenTransfer,
    NFTTransfer,
    Transfer,
}
