use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint_x: Box<Account<'info, Mint>>,
    pub mint_y: Box<Account<'info, Mint>>,

    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [
            b"config",
            mint_x.key().as_ref(),
            mint_y.key().as_ref(),
            config.seed.to_le_bytes().as_ref(),
        ],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,

    // Needed to read the LP supply and initialise the constant-product curve
    #[account(
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub mint_lp: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_y: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_x,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_x: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_y,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_y: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, is_x: bool, amount: u64, minimum: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount > 0, AmmError::InvalidAmount);

        let mut curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            self.mint_lp.supply,
            self.config.fee,
            None, // None defaults to precision 1_000_000 (6 decimal places)
        )
        .map_err(AmmError::from)?;

        let pair = if is_x {
            LiquidityPair::X
        } else {
            LiquidityPair::Y
        };

        let result = curve.swap(pair, amount, minimum).map_err(AmmError::from)?;

        require!(result.deposit > 0, AmmError::InvalidAmount);
        require!(result.withdraw > 0, AmmError::InvalidAmount);

        // is_x=true : user deposits X and receives Y
        // is_x=false: user deposits Y and receives X
        self.deposit_to_vault(is_x, result.deposit)?;
        self.withdraw_from_vault(is_x, result.withdraw)
    }

    fn deposit_to_vault(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (cpi_accounts, decimals) = if is_x {
            (
                TransferChecked {
                    from: self.user_x.to_account_info(),
                    mint: self.mint_x.to_account_info(),
                    to: self.vault_x.to_account_info(),
                    authority: self.user.to_account_info(),
                },
                self.mint_x.decimals,
            )
        } else {
            (
                TransferChecked {
                    from: self.user_y.to_account_info(),
                    mint: self.mint_y.to_account_info(),
                    to: self.vault_y.to_account_info(),
                    authority: self.user.to_account_info(),
                },
                self.mint_y.decimals,
            )
        };

        let cpi_ctx = CpiContext::new(self.token_program.key(), cpi_accounts);
        transfer_checked(cpi_ctx, amount, decimals)
    }

    fn withdraw_from_vault(&mut self, is_x: bool, amount: u64) -> Result<()> {
        // Opposite direction of the deposit: depositing X means withdrawing Y
        let (cpi_accounts, decimals) = if is_x {
            (
                TransferChecked {
                    from: self.vault_y.to_account_info(),
                    mint: self.mint_y.to_account_info(),
                    to: self.user_y.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                self.mint_y.decimals,
            )
        } else {
            (
                TransferChecked {
                    from: self.vault_x.to_account_info(),
                    mint: self.mint_x.to_account_info(),
                    to: self.user_x.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                self.mint_x.decimals,
            )
        };

        let seed_bytes = self.config.seed.to_le_bytes();
        let mint_x_key = self.mint_x.key();
        let mint_y_key = self.mint_y.key();
        let seeds = [
            b"config".as_ref(),
            mint_x_key.as_ref(),
            mint_y_key.as_ref(),
            seed_bytes.as_ref(),
            &[self.config.config_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(self.token_program.key(), cpi_accounts, signer_seeds);
        transfer_checked(cpi_ctx, amount, decimals)
    }
}
