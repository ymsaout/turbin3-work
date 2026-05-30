#![allow(unexpected_cfgs, deprecated, clippy::too_many_arguments)]

use anchor_lang::prelude::*;

declare_id!("8jV53ChBtfkidsc3k8LmyHn9xvqSZVM13qCvhCkFD99e");

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

#[program]
pub mod nft_staking {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        rewards_basis_points: u16,
        freeze_period: u32,
    ) -> Result<()> {
        ctx.accounts
            .initialize(rewards_basis_points, freeze_period, &ctx.bumps)
    }

    pub fn create_collection(
        ctx: Context<CreateCollection>,
        name: String,
        uri: String,
    ) -> Result<()> {
        ctx.accounts.create_collection(name, uri, &ctx.bumps)
    }

    pub fn mint_asset(ctx: Context<MintAsset>, name: String, uri: String) -> Result<()> {
        ctx.accounts.mint_asset(name, uri, &ctx.bumps)
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        ctx.accounts.stake(ctx.bumps.update_authority)
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        ctx.accounts.unstake(ctx.bumps.update_authority)
    }
}
