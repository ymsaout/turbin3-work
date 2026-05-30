use anchor_lang::prelude::*;
use mpl_core::{instructions::CreateV2CpiBuilder, ID as MPL_CORE_ID};

#[derive(Accounts)]
pub struct MintAsset<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub asset: Signer<'info>,

    /// CHECK: collection passée directement au CPI mpl-core
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: PDA de signature uniquement — seeds vérifiées par la contrainte
    #[account(
        seeds = [b"collection_authority", collection.key().as_ref()],
        bump,
    )]
    pub collection_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: adresse vérifiée via la contrainte address
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
}

impl<'info> MintAsset<'info> {
    pub fn mint_asset(
        &self,
        name: String,
        uri: String,
        bumps: &MintAssetBumps,
    ) -> Result<()> {
        let collection_key = self.collection.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"collection_authority",
            collection_key.as_ref(),
            &[bumps.collection_authority],
        ]];

        CreateV2CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .authority(Some(&self.collection_authority.to_account_info()))
            .payer(&self.user.to_account_info())
            .owner(Some(&self.user.to_account_info()))
            .update_authority(None)
            .system_program(&self.system_program.to_account_info())
            .name(name)
            .uri(uri)
            .invoke_signed(signer_seeds)?;

        Ok(())
    }
}
