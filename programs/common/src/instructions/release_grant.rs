use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::get_associated_token_address,
    token::{Token, TokenAccount},
};

// [DEPENDENCY] Imports transfer_asset from programs/common/src/instructions/withdraw_grant.rs
use super::withdraw_grant::transfer_asset;
use crate::{
    constants::*,
    error::ErrorCode,
    state::{Asset, Grant, Pool},
};

#[event]
pub struct GrantFundsReleased {
    pub grant: Pubkey,
    pub pool: Pubkey,
    pub asset: Asset,
    pub amount: u64,
}

/// Anyone may return an expired or revoked grant's unused funds, only to its original pool.
#[derive(Accounts)]
pub struct ReleaseGrant<'info> {
    #[account(mut, has_one = pool @ ErrorCode::WrongPool)]
    pub grant: Account<'info, Grant>,
    pub pool: Account<'info, Pool>,
    #[account(mut, seeds = [VAULT_SEED, grant.key().as_ref()], bump)]
    pub vault: SystemAccount<'info>,
    #[account(mut, seeds = [VAULT_SEED, pool.key().as_ref()], bump)]
    pub pool_vault: SystemAccount<'info>,
    #[account(mut, constraint = vault_token.key() == get_associated_token_address(&vault.key(), &vault_token.mint) @ ErrorCode::WrongVault)]
    pub vault_token: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub pool_token: Option<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_release_grant(ctx: Context<ReleaseGrant>) -> Result<()> {
    release_funds(&ctx.accounts, ctx.bumps.vault)
}

/// Shared with voted revocation so cancellation and returning money happen atomically.
pub(crate) fn release_funds(accounts: &ReleaseGrant, vault_bump: u8) -> Result<()> {
    let grant = &accounts.grant;
    require!(
        grant.revoked || Clock::get()?.unix_timestamp >= grant.expires_at,
        ErrorCode::GrantActive
    );
    let (amount, source_vault, destination) = match grant.asset {
        // Retain the account's rent and grant history; repeated returns then release zero.
        Asset::Sol => (
            grant
                .get_lamports()
                .saturating_sub(Rent::get()?.minimum_balance(grant.to_account_info().data_len())),
            grant.to_account_info(),
            accounts.pool_vault.to_account_info(),
        ),
        Asset::Spl { mint } => {
            let source = accounts
                .vault_token
                .as_ref()
                .ok_or(ErrorCode::MissingTokenAccount)?;
            let destination = accounts
                .pool_token
                .as_ref()
                .ok_or(ErrorCode::MissingTokenAccount)?;
            require_keys_eq!(
                destination.owner,
                accounts.pool_vault.key(),
                ErrorCode::WrongVault
            );
            require_keys_eq!(destination.mint, mint, ErrorCode::WrongMint);
            (
                source.amount,
                accounts.vault.to_account_info(),
                destination.to_account_info(),
            )
        }
    };
    let grant_key = grant.key();
    let seeds: &[&[&[u8]]] = &[&[VAULT_SEED, grant_key.as_ref(), &[vault_bump]]];
    if amount > 0 {
        transfer_asset(
            grant.asset,
            amount,
            &source_vault,
            accounts.vault_token.as_ref(),
            destination,
            seeds,
        )?;
    }
    emit!(GrantFundsReleased {
        grant: grant_key,
        pool: accounts.pool.key(),
        asset: grant.asset,
        amount
    });
    Ok(())
}
