use anchor_lang::prelude::*;

#[error_code]
pub enum StakingError {
    #[msg("Invalid update authority")]
    InvalidUpdateAuthority,
    #[msg("Invalid owner")]
    InvalidOwner,
    #[msg("Asset already staked")]
    AlreadyStaked,
    #[msg("Asset not staked")]
    NotStaked,
    #[msg("Freeze period has not elapsed yet")]
    FreezePeriodNotElapsed,
    #[msg("Invalid timestamp")]
    InvalidTimestamp,
}
