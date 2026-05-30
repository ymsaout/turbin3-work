use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    mint_to, transfer_checked, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
};
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

use crate::errors::MarketplaceError;
use crate::state::{Listing, Marketplace};

#[derive(Accounts)]
pub struct BuyWithToken<'info> {
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
        constraint = listing.payment_mint == Some(payment_mint.key()) @ MarketplaceError::InvalidPaymentMint,
        seeds = [b"listing", marketplace.key().as_ref(), asset.key().as_ref()],
        bump = listing.bump,
    )]
    pub listing: Account<'info, Listing>,

    pub marketplace: Account<'info, Marketplace>,

    #[account(
        seeds = [b"treasury", marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,

    // Pas de contraintes ATA → pas de seeds dans l'IDL → Anchor ne résout pas ces comptes
    // automatiquement. Ils doivent être pré-créés et passés explicitement par le client.
    #[account(mut)]
    pub treasury_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub taker_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub maker_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [b"rewards_mint", marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
    )]
    pub rewards_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub taker_rewards_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,

    /// CHECK: adresse vérifiée via la contrainte address
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> BuyWithToken<'info> {
    pub fn buy_with_token(&self) -> Result<()> {
        let price = self.listing.price;
        let decimals = self.payment_mint.decimals;

        let fee = (price as u128)
            .checked_mul(self.marketplace.fee as u128)
            .unwrap()
            .checked_div(10_000)
            .unwrap() as u64;
        let maker_amount = price.checked_sub(fee).unwrap();

        // Tokens → maker (prix minus frais)
        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.taker_payment_ata.to_account_info(),
                    mint: self.payment_mint.to_account_info(),
                    to: self.maker_payment_ata.to_account_info(),
                    authority: self.taker.to_account_info(),
                },
            ),
            maker_amount,
            decimals,
        )?;

        // Tokens → treasury_ata (frais)
        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.taker_payment_ata.to_account_info(),
                    mint: self.payment_mint.to_account_info(),
                    to: self.treasury_ata.to_account_info(),
                    authority: self.taker.to_account_info(),
                },
            ),
            fee,
            decimals,
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

        // Mint 1 reward token au taker
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
            1_000_000,
        )?;

        Ok(())
    }
}
