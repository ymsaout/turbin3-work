use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Listing {
    pub maker: Pubkey,
    pub asset: Pubkey,
    pub price: u64,
    pub payment_mint: Option<Pubkey>, // None = SOL, Some = SPL token
    pub bump: u8,
}
