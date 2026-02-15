

use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("97yDqjianXxMMztXmfa4XEUaRkpTAviWJXMSSzrk4DGK");

#[program]
pub mod escrow {
    use super::*;

    /// Maker locks `amount` lamports in a vault PDA and creates the escrow state.
    pub fn make(ctx: Context<Make>, amount: u64) -> Result<()> {
        let escrow_state = &mut ctx.accounts.escrow_state;

        escrow_state.maker = ctx.accounts.maker.key();
        escrow_state.amount = amount;
        escrow_state.vault_bump = ctx.bumps.vault;
        escrow_state.escrow_bump = ctx.bumps.escrow_state;

        // Transfer lamports from the maker to the vault PDA.
        let cpi_program = ctx.accounts.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: ctx.accounts.maker.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, amount)?;

        Ok(())
    }

    /// Taker pays the maker and receives the lamports locked in the vault.
    /// The escrow state is closed back to the maker.
    pub fn take(ctx: Context<Take>) -> Result<()> {
        let amount = ctx.accounts.escrow_state.amount;

        // Taker pays the maker.
        let cpi_program = ctx.accounts.system_program.to_account_info();
        let pay_accounts = Transfer {
            from: ctx.accounts.taker.to_account_info(),
            to: ctx.accounts.maker.to_account_info(),
        };
        let pay_ctx = CpiContext::new(cpi_program.clone(), pay_accounts);
        transfer(pay_ctx, amount)?;

        // Vault (PDA) releases the same amount to the taker.
        let seeds = &[
            b"vault",
            ctx.accounts.escrow_state.to_account_info().key.as_ref(),
            &[ctx.accounts.escrow_state.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let release_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.taker.to_account_info(),
        };
        let release_ctx =
            CpiContext::new_with_signer(cpi_program, release_accounts, signer_seeds);
        transfer(release_ctx, amount)?;

        Ok(())
    }

    /// Maker gets a refund of the lamports locked in the vault.
    /// The escrow state is closed back to the maker.
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        let cpi_program = ctx.accounts.system_program.to_account_info();

        let seeds = &[
            b"vault",
            ctx.accounts.escrow_state.to_account_info().key.as_ref(),
            &[ctx.accounts.escrow_state.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let refund_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.maker.to_account_info(),
        };
        let refund_ctx =
            CpiContext::new_with_signer(cpi_program, refund_accounts, signer_seeds);

        // Refund everything that is currently in the vault.
        let amount = ctx.accounts.vault.lamports();
        transfer(refund_ctx, amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Make<'info> {
    /// Maker providing the lamports and paying for account creation.
    #[account(mut)]
    pub maker: Signer<'info>,
    /// Escrow state PDA, one per maker.
    #[account(
        init,
        payer = maker,
        space = EscrowState::INIT_SPACE,
        seeds = [b"escrow", maker.key().as_ref()],
        bump,
    )]
    pub escrow_state: Account<'info, EscrowState>,
    /// Vault PDA that will hold the locked lamports.
    #[account(
        mut,
        seeds = [b"vault", escrow_state.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Take<'info> {
    /// Taker paying the maker and receiving the locked lamports.
    #[account(mut)]
    pub taker: Signer<'info>,
    /// Maker that created the escrow.
    #[account(mut)]
    pub maker: SystemAccount<'info>,
    /// Escrow state for this maker.
    #[account(
        mut,
        seeds = [b"escrow", maker.key().as_ref()],
        bump = escrow_state.escrow_bump,
        has_one = maker,
        close = maker,
    )]
    pub escrow_state: Account<'info, EscrowState>,
    /// Vault PDA holding the locked lamports.
    #[account(
        mut,
        seeds = [b"vault", escrow_state.key().as_ref()],
        bump = escrow_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Refund<'info> {
    /// Maker requesting the refund.
    #[account(mut)]
    pub maker: Signer<'info>,
    /// Escrow state for this maker.
    #[account(
        mut,
        seeds = [b"escrow", maker.key().as_ref()],
        bump = escrow_state.escrow_bump,
        has_one = maker,
        close = maker,
    )]
    pub escrow_state: Account<'info, EscrowState>,
    /// Vault PDA holding the locked lamports.
    #[account(
        mut,
        seeds = [b"vault", escrow_state.key().as_ref()],
        bump = escrow_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct EscrowState {
    pub maker: Pubkey,
    pub amount: u64,
    pub vault_bump: u8,
    pub escrow_bump: u8,
}

impl Space for EscrowState {
    const INIT_SPACE: usize = 8  // discriminator
        + 32 // maker
        + 8  // amount
        + 1  // vault_bump
        + 1; // escrow_bump
}
