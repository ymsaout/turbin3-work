#![allow(unexpected_cfgs, deprecated, clippy::too_many_arguments)]

use anchor_lang::prelude::*;

declare_id!("3sm8PuFRFSXx65L1X5GHJT6mtdsqwpKd1oZvU7vsJMp7");

pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

#[program]
pub mod nft_marketplace {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, name: String, fee: u16) -> Result<()> {
        ctx.accounts.initialize(name, fee, &ctx.bumps)
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

    pub fn list(
        ctx: Context<List>,
        price: u64,
        payment_mint: Option<Pubkey>,
    ) -> Result<()> {
        ctx.accounts.list(price, payment_mint, &ctx.bumps)
    }

    pub fn delist(ctx: Context<Delist>) -> Result<()> {
        ctx.accounts.delist()
    }

    pub fn buy(ctx: Context<Buy>) -> Result<()> {
        ctx.accounts.buy()
    }

    pub fn buy_with_token(ctx: Context<BuyWithToken>) -> Result<()> {
        ctx.accounts.buy_with_token()
    }

    pub fn make_offer(ctx: Context<MakeOffer>, amount: u64) -> Result<()> {
        ctx.accounts.make_offer(amount, &ctx.bumps)
    }

    pub fn accept_offer(ctx: Context<AcceptOffer>) -> Result<()> {
        ctx.accounts.accept_offer()
    }

    pub fn cancel_offer(ctx: Context<CancelOffer>) -> Result<()> {
        ctx.accounts.cancel_offer()
    }
}
