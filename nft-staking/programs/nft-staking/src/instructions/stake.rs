use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{AddPluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{
        Attribute, Attributes, FreezeDelegate, Plugin, PluginAuthority, PluginType,
        UpdateAuthority,
    },
    ID as MPL_CORE_ID,
};

use crate::errors::StakingError;
use crate::state::Config;

#[derive(Accounts)]
pub struct Stake<'info> {
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

    pub system_program: Program<'info, System>,

    /// CHECK: adresse vérifiée via la contrainte address
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
}

impl<'info> Stake<'info> {
    pub fn stake(&self, update_authority_bump: u8) -> Result<()> {
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

        // Récupère le plugin Attributes s'il existe déjà sur l'asset
        let attributes_fetched: Option<Attributes> = fetch_plugin::<BaseAssetV1, Attributes>(
            &self.asset.to_account_info(),
            PluginType::Attributes,
        )
        .ok()
        .map(|(_, attrs, _)| attrs);

        // Construit la liste d'attributs mise à jour
        let mut attributes_list: Vec<Attribute> = Vec::new();

        if let Some(existing) = &attributes_fetched {
            for attr in &existing.attribute_list {
                if attr.key == "staked" {
                    require!(attr.value == "false", StakingError::AlreadyStaked);
                } else if attr.key != "staked_at" {
                    attributes_list.push(attr.clone());
                }
            }
        }

        let clock = Clock::get()?;
        attributes_list.push(Attribute {
            key: "staked".to_string(),
            value: "true".to_string(),
        });
        attributes_list.push(Attribute {
            key: "staked_at".to_string(),
            value: clock.unix_timestamp.to_string(),
        });

        let collection_key = self.collection.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"update_authority",
            collection_key.as_ref(),
            &[update_authority_bump],
        ]];

        // Ajoute ou met à jour le plugin Attributes (authority-managed → invoke_signed)
        if attributes_fetched.is_none() {
            AddPluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
                .asset(&self.asset.to_account_info())
                .collection(Some(&self.collection.to_account_info()))
                .payer(&self.owner.to_account_info())
                .authority(Some(&self.update_authority.to_account_info()))
                .system_program(&self.system_program.to_account_info())
                .plugin(Plugin::Attributes(Attributes {
                    attribute_list: attributes_list,
                }))
                .init_authority(PluginAuthority::UpdateAuthority)
                .invoke_signed(signer_seeds)?;
        } else {
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
        }

        // Ajoute FreezeDelegate (owner-managed → invoke sans signer seeds)
        // init_authority = UpdateAuthority pour que le programme puisse débloquer au unstake
        AddPluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.owner.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
            .init_authority(PluginAuthority::UpdateAuthority)
            .invoke()?;

        Ok(())
    }
}
