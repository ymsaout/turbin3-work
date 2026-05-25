use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{burn, transfer_checked, Burn, Mint, Token, TokenAccount, TransferChecked},
};
use constant_product_curve::ConstantProduct;

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

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

    #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub mint_lp: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_y: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_y: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_lp: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64, min_x: u64, min_y: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount > 0, AmmError::InvalidAmount);

        let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
            self.vault_x.amount,
            self.vault_y.amount,
            self.mint_lp.supply,
            amount,
            1_000_000,
        )
        .map_err(AmmError::from)?;

        require!(amounts.x >= min_x, AmmError::InsufficientTokenX);
        require!(amounts.y >= min_y, AmmError::InsufficientTokenY);

        self.withdraw_token(true, amounts.x)?;
        self.withdraw_token(false, amounts.y)?;
        self.burn_lp_tokens(amount)
    }

    fn withdraw_token(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let (cpi_accounts, decimals) = if is_x {
            (
                TransferChecked {
                    from: self.vault_x.to_account_info(),
                    mint: self.mint_x.to_account_info(),
                    to: self.user_x.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                self.mint_x.decimals,
            )
        } else {
            (
                TransferChecked {
                    from: self.vault_y.to_account_info(),
                    mint: self.mint_y.to_account_info(),
                    to: self.user_y.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                self.mint_y.decimals,
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

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer_checked(cpi_ctx, amount, decimals)
    }

    fn burn_lp_tokens(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Burn {
            mint: self.mint_lp.to_account_info(),
            from: self.user_lp.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        burn(cpi_ctx, amount)
    }
}
