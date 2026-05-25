use anchor_lang::prelude::*;

#[account]
pub struct Config {
    pub seed: u64,
    pub authority: Option<Pubkey>,
    pub mint_x: Pubkey,
    pub mint_y: Pubkey,
    pub fee: u16,
    pub locked: bool,
    pub config_bump: u8,
    pub lp_bump: u8,
}

impl Space for Config {
    const INIT_SPACE: usize = 8   // discriminator
        + 8                       // seed u64
        + 1 + 32                  // authority Option<Pubkey>
        + 32                      // mint_x
        + 32                      // mint_y
        + 2                       // fee u16
        + 1                       // locked bool
        + 1                       // config_bump
        + 1;                      // lp_bump
}
