use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Offer {
    pub buyer: Pubkey,
    pub asset: Pubkey,
    pub amount: u64,     // SOL lamports escrowed in the vault
    pub bump: u8,
    pub vault_bump: u8,  // bump du SystemAccount qui tient les lamports
}
