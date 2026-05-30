use anchor_lang::{prelude::*, system_program};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface},
};
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

use crate::errors::MarketplaceError;
use crate::state::{Listing, Marketplace};

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    /// CHECK: validated via has_one on listing
    #[account(mut)]
    pub maker: UncheckedAccount<'info>,

    /// CHECK: validé par le CPI mpl-core lors du transfert
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: collection optionnelle — passer system_program si absente
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,

    #[account(
        mut,
        close = maker,
        has_one = maker @ MarketplaceError::InvalidMaker,
        has_one = asset @ MarketplaceError::InvalidAsset,
        constraint = listing.payment_mint.is_none() @ MarketplaceError::WrongPaymentMethod,
        seeds = [b"listing", marketplace.key().as_ref(), asset.key().as_ref()],
        bump = listing.bump,
    )]
    pub listing: Account<'info, Listing>,

    pub marketplace: Account<'info, Marketplace>,

    #[account(
        mut,
        seeds = [b"treasury", marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [b"rewards_mint", marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = rewards_mint,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_rewards_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// CHECK: adresse vérifiée via la contrainte address
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Buy<'info> {
    pub fn buy(&self) -> Result<()> {
        let price = self.listing.price;
        let fee = (price as u128)
            .checked_mul(self.marketplace.fee as u128)
            .unwrap()
            .checked_div(10_000)
            .unwrap() as u64;
        let maker_amount = price.checked_sub(fee).unwrap();

        // SOL → maker (prix minus frais)
        system_program::transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                system_program::Transfer {
                    from: self.taker.to_account_info(),
                    to: self.maker.to_account_info(),
                },
            ),
            maker_amount,
        )?;

        // SOL → treasury (frais)
        system_program::transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                system_program::Transfer {
                    from: self.taker.to_account_info(),
                    to: self.treasury.to_account_info(),
                },
            ),
            fee,
        )?;

        // NFT : listing PDA → taker
        let marketplace_key = self.marketplace.key();
        let asset_key = self.listing.asset;
        let listing_seeds: &[&[&[u8]]] = &[&[
            b"listing",
            marketplace_key.as_ref(),
            asset_key.as_ref(),
            &[self.listing.bump],
        ]];

        let mpl_core_ai = self.mpl_core_program.to_account_info();
        let asset_ai = self.asset.to_account_info();
        let collection_ai = self.collection.to_account_info();
        let taker_ai = self.taker.to_account_info();
        let listing_ai = self.listing.to_account_info();
        let system_ai = self.system_program.to_account_info();

        let is_system = collection_ai.key() == System::id();

        let mut builder = TransferV1CpiBuilder::new(&mpl_core_ai);
        builder
            .asset(&asset_ai)
            .payer(&taker_ai)
            .authority(Some(&listing_ai))
            .new_owner(&taker_ai)
            .system_program(Some(&system_ai));

        if !is_system {
            builder.collection(Some(&collection_ai));
        }

        builder.invoke_signed(listing_seeds)?;

        // Mint 1 token de reward au taker
        let name_bytes = self.marketplace.name.as_bytes();
        let marketplace_seeds: &[&[&[u8]]] = &[&[
            b"marketplace",
            name_bytes,
            &[self.marketplace.bump],
        ]];

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.rewards_mint.to_account_info(),
                    to: self.taker_rewards_ata.to_account_info(),
                    authority: self.marketplace.to_account_info(),
                },
                marketplace_seeds,
            ),
            1_000_000, // 1 token (6 décimales)
        )?;

        Ok(())
    }
}
