use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface},
};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::UpdatePluginV1CpiBuilder,
    types::{
        Attribute, Attributes, FreezeDelegate, Plugin, PluginType, UpdateAuthority,
    },
    ID as MPL_CORE_ID,
};

use crate::constants::SECONDS_PER_DAY;
use crate::errors::StakingError;
use crate::state::Config;

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        seeds = [b"config", collection.key().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,

    /// CHECK: owned by MPL Core; owner et update_authority validés dans le body
    #[account(mut, owner = MPL_CORE_ID)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: owned by MPL Core; update_authority validée dans le body
    #[account(mut, owner = MPL_CORE_ID)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: PDA utilisé uniquement pour signer — seeds vérifiées par la contrainte
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"rewards_mint", collection.key().as_ref()],
        bump = config.rewards_bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = rewards_mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
    )]
    pub user_rewards_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,

    /// CHECK: adresse vérifiée via la contrainte address
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
}

impl<'info> Unstake<'info> {
    pub fn unstake(&self, update_authority_bump: u8) -> Result<()> {
        // Valide le owner et l'update_authority de l'asset
        // deserialize_reader lit seulement les champs du struct et ignore les plugin data
        let asset = BaseAssetV1::deserialize_reader(&mut self.asset.data.borrow().as_ref())
            .map_err(|_| error!(StakingError::InvalidOwner))?;

        require!(asset.owner == self.owner.key(), StakingError::InvalidOwner);
        require!(
            asset.update_authority
                == UpdateAuthority::Collection(self.collection.key()),
            StakingError::InvalidUpdateAuthority
        );

        // Valide l'update_authority de la collection
        let collection = BaseCollectionV1::deserialize_reader(&mut self.collection.data.borrow().as_ref())
            .map_err(|_| error!(StakingError::InvalidUpdateAuthority))?;

        require!(
            collection.update_authority == self.update_authority.key(),
            StakingError::InvalidUpdateAuthority
        );

        // Le plugin Attributes doit exister (asset non staké → erreur)
        let (_, existing_attributes, _) =
            fetch_plugin::<BaseAssetV1, Attributes>(
                &self.asset.to_account_info(),
                PluginType::Attributes,
            )
            .map_err(|_| error!(StakingError::NotStaked))?;

        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp;

        let mut attributes_list: Vec<Attribute> =
            Vec::with_capacity(existing_attributes.attribute_list.len());
        let mut staked_timestamp: i64 = 0;

        for attr in &existing_attributes.attribute_list {
            if attr.key == "staked" {
                require!(attr.value == "true", StakingError::NotStaked);
            } else if attr.key == "staked_at" {
                staked_timestamp = attr
                    .value
                    .parse::<i64>()
                    .map_err(|_| error!(StakingError::InvalidTimestamp))?;

                let elapsed_seconds = current_timestamp
                    .checked_sub(staked_timestamp)
                    .ok_or(error!(StakingError::InvalidTimestamp))?;
                let elapsed_days = elapsed_seconds
                    .checked_div(SECONDS_PER_DAY)
                    .ok_or(error!(StakingError::InvalidTimestamp))?;

                require!(
                    elapsed_days >= self.config.freeze_period as i64,
                    StakingError::FreezePeriodNotElapsed
                );
            } else {
                attributes_list.push(attr.clone());
            }
        }

        // Remet les attributs staking à leur état par défaut
        attributes_list.push(Attribute {
            key: "staked".to_string(),
            value: "false".to_string(),
        });
        attributes_list.push(Attribute {
            key: "staked_at".to_string(),
            value: "0".to_string(),
        });

        let collection_key = self.collection.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"update_authority",
            collection_key.as_ref(),
            &[update_authority_bump],
        ]];

        // Met à jour le plugin Attributes
        UpdatePluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::Attributes(Attributes {
                attribute_list: attributes_list,
            }))
            .invoke_signed(signer_seeds)?;

        // Décongèle l'asset (FreezeDelegate frozen = false)
        UpdatePluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .invoke_signed(signer_seeds)?;

        // Calcule et mint les rewards
        let elapsed_seconds = current_timestamp
            .checked_sub(staked_timestamp)
            .ok_or(error!(StakingError::InvalidTimestamp))?;
        let elapsed_days = elapsed_seconds
            .checked_div(SECONDS_PER_DAY)
            .ok_or(error!(StakingError::InvalidTimestamp))? as u64;

        let rewards_amount = elapsed_days
            .checked_mul(self.config.rewards_basis_points as u64)
            .ok_or(error!(StakingError::InvalidTimestamp))?;

        let config_seeds: &[&[&[u8]]] = &[&[
            b"config",
            collection_key.as_ref(),
            &[self.config.config_bump],
        ]];

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.rewards_mint.to_account_info(),
                    to: self.user_rewards_ata.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                config_seeds,
            ),
            rewards_amount,
        )?;

        Ok(())
    }
}
