use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke_signed, system_instruction},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface},
};
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

use crate::errors::MarketplaceError;
use crate::state::{Listing, Marketplace, Offer};

#[derive(Accounts)]
pub struct AcceptOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    /// CHECK: buyer reçoit le NFT et le remboursement du rent
    #[account(mut)]
    pub buyer: UncheckedAccount<'info>,

    /// CHECK: validé par le CPI mpl-core
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
        seeds = [b"listing", marketplace.key().as_ref(), asset.key().as_ref()],
        bump = listing.bump,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        mut,
        close = buyer,
        has_one = buyer @ MarketplaceError::InvalidBuyer,
        has_one = asset @ MarketplaceError::InvalidAsset,
        seeds = [b"offer", asset.key().as_ref(), offer.buyer.as_ref()],
        bump = offer.bump,
    )]
    pub offer: Account<'info, Offer>,

    /// Vault SOL de l'offre — vidé via invoke_signed avant la clôture
    #[account(
        mut,
        seeds = [b"offer_vault", asset.key().as_ref(), offer.buyer.as_ref()],
        bump = offer.vault_bump,
    )]
    pub offer_vault: SystemAccount<'info>,

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
        payer = maker,
        associated_token::mint = rewards_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program,
    )]
    pub buyer_rewards_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// CHECK: adresse vérifiée via la contrainte address
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> AcceptOffer<'info> {
    pub fn accept_offer(&self) -> Result<()> {
        let amount = self.offer.amount;
        let fee = (amount as u128)
            .checked_mul(self.marketplace.fee as u128)
            .unwrap()
            .checked_div(10_000)
            .unwrap() as u64;
        let maker_amount = amount.checked_sub(fee).unwrap();

        // Seeds du vault pour invoke_signed
        let asset_key = self.offer.asset;
        let buyer_key = self.offer.buyer;
        let vault_seeds: &[&[&[u8]]] = &[&[
            b"offer_vault",
            asset_key.as_ref(),
            buyer_key.as_ref(),
            &[self.offer.vault_bump],
        ]];

        // Vault → maker (prix minus frais)
        let vault_ai = self.offer_vault.to_account_info();
        let maker_ai = self.maker.to_account_info();
        invoke_signed(
            &system_instruction::transfer(vault_ai.key, maker_ai.key, maker_amount),
            &[
                vault_ai.clone(),
                maker_ai.clone(),
                self.system_program.to_account_info(),
            ],
            vault_seeds,
        )?;

        // Vault → treasury (frais)
        let treasury_ai = self.treasury.to_account_info();
        invoke_signed(
            &system_instruction::transfer(vault_ai.key, treasury_ai.key, fee),
            &[
                vault_ai.clone(),
                treasury_ai.clone(),
                self.system_program.to_account_info(),
            ],
            vault_seeds,
        )?;

        // NFT : listing PDA → buyer
        let marketplace_key = self.marketplace.key();
        let listing_seeds: &[&[&[u8]]] = &[&[
            b"listing",
            marketplace_key.as_ref(),
            asset_key.as_ref(),
            &[self.listing.bump],
        ]];

        let mpl_core_ai = self.mpl_core_program.to_account_info();
        let asset_ai = self.asset.to_account_info();
        let collection_ai = self.collection.to_account_info();
        let buyer_ai = self.buyer.to_account_info();
        let listing_ai = self.listing.to_account_info();
        let system_ai = self.system_program.to_account_info();

        let is_system = collection_ai.key() == System::id();

        let mut builder = TransferV1CpiBuilder::new(&mpl_core_ai);
        builder
            .asset(&asset_ai)
            .payer(&maker_ai)
            .authority(Some(&listing_ai))
            .new_owner(&buyer_ai)
            .system_program(Some(&system_ai));

        if !is_system {
            builder.collection(Some(&collection_ai));
        }

        builder.invoke_signed(listing_seeds)?;

        // Mint 1 reward token au buyer
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
                    to: self.buyer_rewards_ata.to_account_info(),
                    authority: self.marketplace.to_account_info(),
                },
                marketplace_seeds,
            ),
            1_000_000,
        )?;

        Ok(())
    }
}
