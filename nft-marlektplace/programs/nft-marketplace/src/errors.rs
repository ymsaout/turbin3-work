use anchor_lang::prelude::*;

#[error_code]
pub enum MarketplaceError {
    #[msg("Fee must be between 0 and 10000 basis points")]
    InvalidFee,
    #[msg("Marketplace name exceeds 32 characters")]
    NameTooLong,
    #[msg("Invalid maker account")]
    InvalidMaker,
    #[msg("Invalid asset account")]
    InvalidAsset,
    #[msg("Invalid payment mint for this listing")]
    InvalidPaymentMint,
    #[msg("This listing requires SOL payment")]
    WrongPaymentMethod,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Invalid offer amount")]
    InvalidOfferAmount,
    #[msg("Invalid buyer account")]
    InvalidBuyer,
}
