use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke_signed, system_instruction},
};

use crate::errors::MarketplaceError;
use crate::state::Offer;

#[derive(Accounts)]
pub struct CancelOffer<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    /// CHECK: adresse de l'asset concerné
    pub asset: UncheckedAccount<'info>,

    #[account(
        mut,
        close = buyer,
        has_one = buyer @ MarketplaceError::InvalidBuyer,
        has_one = asset @ MarketplaceError::InvalidAsset,
        seeds = [b"offer", asset.key().as_ref(), buyer.key().as_ref()],
        bump = offer.bump,
    )]
    pub offer: Account<'info, Offer>,

    /// Vault SOL de l'offre à rembourser
    #[account(
        mut,
        seeds = [b"offer_vault", asset.key().as_ref(), buyer.key().as_ref()],
        bump = offer.vault_bump,
    )]
    pub offer_vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> CancelOffer<'info> {
    pub fn cancel_offer(&self) -> Result<()> {
        let asset_key = self.offer.asset;
        let buyer_key = self.offer.buyer;
        let vault_seeds: &[&[&[u8]]] = &[&[
            b"offer_vault",
            asset_key.as_ref(),
            buyer_key.as_ref(),
            &[self.offer.vault_bump],
        ]];

        // Vault → buyer : rembourse tous les lamports escrowés
        let vault_lamports = self.offer_vault.to_account_info().lamports();
        let vault_ai = self.offer_vault.to_account_info();
        let buyer_ai = self.buyer.to_account_info();

        invoke_signed(
            &system_instruction::transfer(vault_ai.key, buyer_ai.key, vault_lamports),
            &[
                vault_ai.clone(),
                buyer_ai.clone(),
                self.system_program.to_account_info(),
            ],
            vault_seeds,
        )?;

        // offer data account fermé → buyer via Anchor's close = buyer
        Ok(())
    }
}
