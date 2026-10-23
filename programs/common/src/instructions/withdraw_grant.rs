use anchor_lang::{prelude::*, system_program};
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    constants::*,
    error::ErrorCode,
    state::{Asset, Grant, Pool},
};

#[event]
pub struct GrantWithdrawn {
    pub grant: Pubkey,
    pub pool: Pubkey,
    pub destination: Pubkey,
    pub asset: Asset,
    pub amount: u64,
    pub total_withdrawn: u64,
}

/// A grantee spends from their grant without a new vote. The program enforces who can
/// sign, the ceiling, the expiry, and revocation.
#[derive(Accounts)]
pub struct WithdrawGrant<'info> {
    pub grantee: Signer<'info>,
    #[account(
        mut,
        has_one = grantee @ ErrorCode::Unauthorized,
        has_one = pool @ ErrorCode::WrongPool
    )]
    pub grant: Account<'info, Grant>,
    pub pool: Account<'info, Pool>,
    /// Authority for this grant's reserved SPL tokens; SOL sits in the grant account itself.
    #[account(mut, seeds = [VAULT_SEED, grant.key().as_ref()], bump)]
    pub vault: SystemAccount<'info>,
    /// CHECK: a wallet for SOL or a token account for SPL; the system or token program validates it.
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,
    /// SPL grants use one canonical token account so the reserved balance cannot be substituted.
    #[account(mut, constraint = vault_token.key() == get_associated_token_address(&vault.key(), &vault_token.mint) @ ErrorCode::WrongVault)]
    pub vault_token: Option<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_withdraw_grant(ctx: Context<WithdrawGrant>, amount: u64) -> Result<()> {
    let a = &ctx.accounts;
    let grant = &a.grant;
    require!(!grant.revoked, ErrorCode::GrantRevoked);
    require!(
        Clock::get()?.unix_timestamp < grant.expires_at,
        ErrorCode::GrantExpired
    );
    let total_withdrawn = grant
        .withdrawn
        .checked_add(amount)
        .filter(|total| *total <= grant.ceiling)
        .ok_or(ErrorCode::CeilingExceeded)?;

    let grant_key = grant.key();
    let seeds: &[&[&[u8]]] = &[&[VAULT_SEED, grant_key.as_ref(), &[ctx.bumps.vault]]];
    let asset = grant.asset;
    let source = match asset {
        Asset::Sol => grant.to_account_info(),
        Asset::Spl { .. } => a.vault.to_account_info(),
    };
    transfer_asset(
        asset,
        amount,
        &source,
        a.vault_token.as_ref(),
        a.destination.to_account_info(),
        seeds,
    )?;

    let pool = a.pool.key();
    let destination = a.destination.key();
    ctx.accounts.grant.withdrawn = total_withdrawn;
    emit!(GrantWithdrawn {
        grant: ctx.accounts.grant.key(),
        pool,
        destination,
        asset,
        amount,
        total_withdrawn,
    });
    Ok(())
}

/// Shared payout path for reservations, withdrawals and returns. CPIs are atomic with
/// grant state changes; SPL sources must belong to the signing vault and match the asset.
pub(crate) fn transfer_asset<'info>(
    asset: Asset,
    amount: u64,
    vault: &AccountInfo<'info>,
    vault_token: Option<&Account<'info, TokenAccount>>,
    destination: AccountInfo<'info>,
    seeds: &[&[&[u8]]],
) -> Result<()> {
    match asset {
        Asset::Sol => {
            require!(vault.lamports() >= amount, ErrorCode::InsufficientFunds);
            if vault.owner == &crate::ID {
                // Grant accounts hold their own SOL reserve in addition to account rent.
                vault.sub_lamports(amount)?;
                destination.add_lamports(amount)?;
                return Ok(());
            }
            let cpi_accounts = system_program::Transfer {
                from: vault.to_account_info(),
                to: destination,
            };
            system_program::transfer(
                CpiContext::new_with_signer(system_program::ID, cpi_accounts, seeds),
                amount,
            )?;
        }
        Asset::Spl { mint } => {
            let vault_token = vault_token.ok_or(ErrorCode::MissingTokenAccount)?;
            require_keys_eq!(vault_token.mint, mint, ErrorCode::WrongMint);
            require_keys_eq!(vault_token.owner, vault.key(), ErrorCode::WrongVault);
            require!(vault_token.amount >= amount, ErrorCode::InsufficientFunds);
            let cpi_accounts = Transfer {
                from: vault_token.to_account_info(),
                to: destination,
                authority: vault.to_account_info(),
            };
            token::transfer(
                CpiContext::new_with_signer(token::ID, cpi_accounts, seeds),
                amount,
            )?;
        }
    }

    Ok(())
}
