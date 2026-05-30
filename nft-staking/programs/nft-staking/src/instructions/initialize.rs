use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use mpl_core::{accounts::BaseCollectionV1, ID as MPL_CORE_ID};

use crate::errors::StakingError;
use crate::state::Config;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config", collection.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    /// CHECK: owned by MPL Core; update_authority validated in body
    #[account(owner = MPL_CORE_ID)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: PDA utilisé uniquement pour signer — seeds vérifiées par la contrainte
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = admin,
        mint::decimals = 6,
        mint::authority = config,
        seeds = [b"rewards_mint", collection.key().as_ref()],
        bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(
        &mut self,
        rewards_basis_points: u16,
        freeze_period: u32,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        let collection =
            BaseCollectionV1::try_from_slice(&self.collection.data.borrow())
                .map_err(|_| error!(StakingError::InvalidUpdateAuthority))?;

        require!(
            collection.update_authority == self.update_authority.key(),
            StakingError::InvalidUpdateAuthority
        );

        self.config.set_inner(Config {
            rewards_basis_points,
            freeze_period,
            config_bump: bumps.config,
            rewards_bump: bumps.rewards_mint,
        });
        Ok(())
    }
}
