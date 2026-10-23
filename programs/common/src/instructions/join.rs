use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    constants::*,
    state::{Member, Treasury},
};

#[derive(Accounts)]
pub struct Join<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [TREASURY_SEED], bump = treasury.bump)]
    pub treasury: Account<'info, Treasury>,
    #[account(
        init,
        payer = owner,
        space = 8 + Member::INIT_SPACE,
        seeds = [MEMBER_SEED, owner.key().as_ref()],
        bump
    )]
    pub member: Account<'info, Member>,
    #[account(mut)]
    pub owner_cmn: Account<'info, TokenAccount>,
    #[account(mut, seeds = [STAKE_VAULT_SEED], bump)]
    pub stake_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// Stake CMN and count the member only if the entire join transaction succeeds.
pub fn handle_join(ctx: Context<Join>) -> Result<()> {
    let cpi_accounts = Transfer {
        from: ctx.accounts.owner_cmn.to_account_info(),
        to: ctx.accounts.stake_vault.to_account_info(),
        authority: ctx.accounts.owner.to_account_info(),
    };
    token::transfer(
        CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts),
        ctx.accounts.treasury.min_stake,
    )?;

    ctx.accounts.member.set_inner(Member {
        owner: ctx.accounts.owner.key(),
        locked_until: 0,
        bump: ctx.bumps.member,
    });
    ctx.accounts.treasury.member_count += 1;
    Ok(())
}
