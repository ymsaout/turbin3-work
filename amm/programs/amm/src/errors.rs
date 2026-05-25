use anchor_lang::prelude::*;
use constant_product_curve::CurveError;

#[error_code]
pub enum AmmError {
    #[msg("Pool is locked")]
    PoolLocked,
    #[msg("Invalid amount: must be greater than zero")]
    InvalidAmount,
    #[msg("Slippage limit exceeded")]
    SlippageExceeded,
    #[msg("Insufficient token X")]
    InsufficientTokenX,
    #[msg("Insufficient token Y")]
    InsufficientTokenY,
}

impl From<CurveError> for AmmError {
    fn from(e: CurveError) -> Self {
        match e {
            CurveError::SlippageLimitExceeded => AmmError::SlippageExceeded,
            CurveError::ZeroBalance => AmmError::InvalidAmount,
            CurveError::InsufficientBalance => AmmError::InvalidAmount,
            _ => AmmError::InvalidAmount,
        }
    }
}
