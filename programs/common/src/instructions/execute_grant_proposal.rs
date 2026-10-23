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
    state::{Asset, Grant, Pool, Proposal, ProposalAction},
};

#[event]
pub struct GrantApproved {
    pub grant: Pubkey,
    pub pool: Pubkey,
    pub grantee: Pubkey,
    pub asset: Asset,
    pub ceiling: u64,
    pub expires_at: i64,
}

/// Anyone can execute a passed ApproveGrant proposal. The executor pays the grant account's rent.
#[derive(Accounts)]
pub struct ExecuteGrantProposal<'info> {
    #[account(mut)]
    pub executor: Signer<'info>,
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    pub pool: Account<'info, Pool>,
    #[account(
        init,
        payer = executor,
        space = 8 + Grant::INIT_SPACE,
        seeds = [GRANT_SEED, proposal.key().as_ref()],
        bump
    )]
    pub grant: Account<'info, Grant>,
    #[account(mut, seeds = [VAULT_SEED, pool.key().as_ref()], bump)]
    pub vault: SystemAccount<'info>,
    #[account(mut, seeds = [VAULT_SEED, grant.key().as_ref()], bump)]
    pub grant_vault: SystemAccount<'info>,
    #[account(mut)]
    pub vault_token: Option<Account<'info, TokenAccount>>,
    /// Created with the standard associated-token instruction, optionally in this same transaction.
    #[account(mut, constraint = grant_token.key() == get_associated_token_address(&grant_vault.key(), &grant_token.mint) @ ErrorCode::WrongVault)]
    pub grant_token: Option<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_execute_grant_proposal(ctx: Context<ExecuteGrantProposal>) -> Result<()> {
    let proposal = &ctx.accounts.proposal;
    proposal.require_passed()?;
    let ProposalAction::ApproveGrant {
        pool,
        grantee,
        asset,
        ceiling,
        expires_at,
    } = proposal.action
    else {
        return err!(ErrorCode::WrongProposalType);
    };
    require_keys_eq!(ctx.accounts.pool.key(), pool, ErrorCode::WrongPool);
    require!(
        Clock::get()?.unix_timestamp < expires_at,
        ErrorCode::GrantExpired
    );

    // Reserve the entire ceiling before granting authority. Failed funding rolls back approval.
    let destination = match asset {
        // The program-owned grant account already has rent, so even tiny grants can be reserved.
        Asset::Sol => ctx.accounts.grant.to_account_info(),
        Asset::Spl { mint } => {
            let token = ctx
                .accounts
                .grant_token
                .as_ref()
                .ok_or(ErrorCode::MissingTokenAccount)?;
            require_keys_eq!(token.mint, mint, ErrorCode::WrongMint);
            require_keys_eq!(
                token.owner,
                ctx.accounts.grant_vault.key(),
                ErrorCode::WrongVault
            );
            token.to_account_info()
        }
    };
    let seeds: &[&[&[u8]]] = &[&[VAULT_SEED, pool.as_ref(), &[ctx.bumps.vault]]];
    transfer_asset(
        asset,
        ceiling,
        &ctx.accounts.vault.to_account_info(),
        ctx.accounts.vault_token.as_ref(),
        destination,
        seeds,
    )?;

    let proposal_key = proposal.key();
    ctx.accounts.grant.set_inner(Grant {
        proposal: proposal_key,
        pool,
        grantee,
        asset,
        ceiling,
        withdrawn: 0,
        expires_at,
        revoked: false,
        bump: ctx.bumps.grant,
    });
    ctx.accounts.proposal.executed = true;

    emit!(GrantApproved {
        grant: ctx.accounts.grant.key(),
        pool,
        grantee,
        asset,
        ceiling,
        expires_at,
    });
    Ok(())
}
