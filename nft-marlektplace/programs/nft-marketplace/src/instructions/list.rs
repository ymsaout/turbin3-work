use anchor_lang::prelude::*;
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

use crate::state::{Listing, Marketplace};

#[derive(Accounts)]
pub struct List<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    /// CHECK: validé par le CPI mpl-core lors du transfert
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: collection optionnelle — passer system_program si absente
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,

    #[account(
        init,
        payer = maker,
        space = 8 + Listing::INIT_SPACE,
        seeds = [b"listing", marketplace.key().as_ref(), asset.key().as_ref()],
        bump,
    )]
    pub listing: Account<'info, Listing>,

    pub marketplace: Account<'info, Marketplace>,

    /// CHECK: adresse vérifiée via la contrainte address
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> List<'info> {
    pub fn list(
        &mut self,
        price: u64,
        payment_mint: Option<Pubkey>,
        bumps: &ListBumps,
    ) -> Result<()> {
        self.listing.set_inner(Listing {
            maker: self.maker.key(),
            asset: self.asset.key(),
            price,
            payment_mint,
            bump: bumps.listing,
        });

        // Transfère le NFT de maker → listing PDA (custodial)
        let mpl_core_ai = self.mpl_core_program.to_account_info();
        let asset_ai = self.asset.to_account_info();
        let collection_ai = self.collection.to_account_info();
        let maker_ai = self.maker.to_account_info();
        let listing_ai = self.listing.to_account_info();
        let system_ai = self.system_program.to_account_info();

        let is_system = collection_ai.key() == System::id();

        let mut builder = TransferV1CpiBuilder::new(&mpl_core_ai);
        builder
            .asset(&asset_ai)
            .payer(&maker_ai)
            .authority(Some(&maker_ai))
            .new_owner(&listing_ai)
            .system_program(Some(&system_ai));

        if !is_system {
            builder.collection(Some(&collection_ai));
        }

        builder.invoke()?;

        Ok(())
    }
}
