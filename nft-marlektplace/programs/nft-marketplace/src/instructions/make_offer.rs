use anchor_lang::{prelude::*, system_program};

use crate::errors::MarketplaceError;
use crate::state::{Listing, Marketplace, Offer};

#[derive(Accounts)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    /// CHECK: adresse de l'asset concerné par l'offre
    pub asset: UncheckedAccount<'info>,

    /// Vérifie que le listing existe pour cet asset (SOL uniquement)
    #[account(
        seeds = [b"listing", marketplace.key().as_ref(), asset.key().as_ref()],
        bump = listing.bump,
        constraint = listing.payment_mint.is_none() @ MarketplaceError::WrongPaymentMethod,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        init,
        payer = buyer,
        space = 8 + Offer::INIT_SPACE,
        seeds = [b"offer", asset.key().as_ref(), buyer.key().as_ref()],
        bump,
    )]
    pub offer: Account<'info, Offer>,

    /// SystemAccount séparé qui détient les lamports escrowés
    /// (pas de data → compatible avec system_instruction::transfer via invoke_signed)
    #[account(
        mut,
        seeds = [b"offer_vault", asset.key().as_ref(), buyer.key().as_ref()],
        bump,
    )]
    pub offer_vault: SystemAccount<'info>,

    pub marketplace: Account<'info, Marketplace>,
    pub system_program: Program<'info, System>,
}

impl<'info> MakeOffer<'info> {
    pub fn make_offer(&mut self, amount: u64, bumps: &MakeOfferBumps) -> Result<()> {
        require!(amount > 0, MarketplaceError::InvalidOfferAmount);

        self.offer.set_inner(Offer {
            buyer: self.buyer.key(),
            asset: self.asset.key(),
            amount,
            bump: bumps.offer,
            vault_bump: bumps.offer_vault,
        });

        // Escrow : SOL buyer → offer_vault (SystemAccount, aucune data)
        system_program::transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                system_program::Transfer {
                    from: self.buyer.to_account_info(),
                    to: self.offer_vault.to_account_info(),
                },
            ),
            amount,
        )?;

        Ok(())
    }
}
