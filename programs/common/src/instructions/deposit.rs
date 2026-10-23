use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    constants::*,
    error::ErrorCode,
    state::{Asset, Pool},
};

#[event]
pub struct Deposited {
    pub pool: Pubkey,
    pub depositor: Pubkey,
    pub asset: Asset,
    pub amount: u64,
}

/// Anyone can fund a pool. A plain transfer to the vault also works, but only this
/// instruction records a Deposited event.
#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,
    pub pool: Account<'info, Pool>,
    #[account(mut, seeds = [VAULT_SEED, pool.key().as_ref()], bump)]
    pub vault: SystemAccount<'info>,
    /// SPL deposits pass both token accounts for the mint; SOL deposits leave both out.
    #[account(mut)]
    pub depositor_token: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub vault_token: Option<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let a = &ctx.accounts;
    let asset = match (&a.depositor_token, &a.vault_token) {
        (None, None) => {
            let cpi_accounts = system_program::Transfer {
                from: a.depositor.to_account_info(),
                to: a.vault.to_account_info(),
            };
            system_program::transfer(CpiContext::new(system_program::ID, cpi_accounts), amount)?;
            Asset::Sol
        }
        (Some(from), Some(to)) => {
            require_keys_eq!(to.owner, a.vault.key(), ErrorCode::WrongVault);
            let cpi_accounts = Transfer {
                from: from.to_account_info(),
                to: to.to_account_info(),
                authority: a.depositor.to_account_info(),
            };
            token::transfer(CpiContext::new(token::ID, cpi_accounts), amount)?;
            Asset::Spl { mint: to.mint }
        }
        _ => return err!(ErrorCode::MissingTokenAccount),
    };

    emit!(Deposited {
        pool: a.pool.key(),
        depositor: a.depositor.key(),
        asset,
        amount,
    });
    Ok(())
}
