use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub rewards_basis_points: u16,
    pub freeze_period: u32, // in days
    pub config_bump: u8,
    pub rewards_bump: u8,
}
