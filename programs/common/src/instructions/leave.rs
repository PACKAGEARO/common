use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    constants::*,
    error::ErrorCode,
    state::{Member, Treasury},
};

#[derive(Accounts)]
pub struct Leave<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [TREASURY_SEED], bump = treasury.bump)]
    pub treasury: Account<'info, Treasury>,
    #[account(
        mut,
        close = owner,
        seeds = [MEMBER_SEED, owner.key().as_ref()],
        bump = member.bump
    )]
    pub member: Account<'info, Member>,
    #[account(mut)]
    pub owner_cmn: Account<'info, TokenAccount>,
    #[account(mut, seeds = [STAKE_VAULT_SEED], bump)]
    pub stake_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

/// Return an unlocked stake and decrement membership atomically with closing the member account.
pub fn handle_leave(ctx: Context<Leave>) -> Result<()> {
    require!(
        Clock::get()?.unix_timestamp >= ctx.accounts.member.locked_until,
        ErrorCode::StakeLocked
    );

    let seeds: &[&[&[u8]]] = &[&[TREASURY_SEED, &[ctx.accounts.treasury.bump]]];
    let cpi_accounts = Transfer {
        from: ctx.accounts.stake_vault.to_account_info(),
        to: ctx.accounts.owner_cmn.to_account_info(),
        authority: ctx.accounts.treasury.to_account_info(),
    };
    token::transfer(
        CpiContext::new_with_signer(ctx.accounts.token_program.key(), cpi_accounts, seeds),
        ctx.accounts.treasury.min_stake,
    )?;
    ctx.accounts.treasury.member_count -= 1;
    Ok(())
}
