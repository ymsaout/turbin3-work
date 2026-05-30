use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

use crate::errors::MarketplaceError;
use crate::state::Marketplace;

#[derive(Accounts)]
#[instruction(name: String)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + Marketplace::INIT_SPACE,
        seeds = [b"marketplace", name.as_bytes()],
        bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    /// Vault where SOL fees accumulate
    #[account(
        seeds = [b"treasury", marketplace.key().as_ref()],
        bump,
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        init,
        payer = admin,
        mint::decimals = 6,
        mint::authority = marketplace,
        seeds = [b"rewards_mint", marketplace.key().as_ref()],
        bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(
        &mut self,
        name: String,
        fee: u16,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        require!(fee <= 10_000, MarketplaceError::InvalidFee);
        require!(name.len() <= 32, MarketplaceError::NameTooLong);

        self.marketplace.set_inner(Marketplace {
            admin: self.admin.key(),
            fee,
            bump: bumps.marketplace,
            treasury_bump: bumps.treasury,
            rewards_bump: bumps.rewards_mint,
            name,
        });
        Ok(())
    }
}
