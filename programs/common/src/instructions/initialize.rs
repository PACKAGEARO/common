use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{constants::*, error::ErrorCode, program::Common, state::Treasury};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + Treasury::INIT_SPACE,
        seeds = [TREASURY_SEED],
        bump
    )]
    pub treasury: Account<'info, Treasury>,
    pub cmn_mint: Account<'info, Mint>,
    /// Holds members' CMN stakes. Its authority is the treasury PDA, not the vault,
    /// so a proposal can never pay out staked CMN.
    #[account(
        init,
        payer = payer,
        seeds = [STAKE_VAULT_SEED],
        bump,
        token::mint = cmn_mint,
        token::authority = treasury,
    )]
    pub stake_vault: Account<'info, TokenAccount>,
    // Only the upgrade authority can initialize, so nobody can front-run setup with their own config.
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, Common>,
    #[account(constraint = program_data.upgrade_authority_address == Some(payer.key()) @ ErrorCode::Unauthorized)]
    pub program_data: Account<'info, ProgramData>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// The upgrade authority selects the staking mint, stake, duration and quorum percentage once.
pub fn handle_initialize(
    ctx: Context<Initialize>,
    min_stake: u64,
    voting_period: i64,
    quorum_percent: u8,
) -> Result<()> {
    require!(
        min_stake > 0 && voting_period > 0 && (1..=100).contains(&quorum_percent),
        ErrorCode::InvalidConfig
    );

    ctx.accounts.treasury.set_inner(Treasury {
        cmn_mint: ctx.accounts.cmn_mint.key(),
        min_stake,
        voting_period,
        quorum_percent,
        member_count: 0,
        proposal_count: 0,
        pool_count: 0,
        bump: ctx.bumps.treasury,
    });
    Ok(())
}
