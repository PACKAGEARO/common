use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::ErrorCode,
    state::{Member, Proposal, VoteRecord},
};

#[derive(Accounts)]
pub struct CastVote<'info> {
    #[account(mut)]
    pub voter: Signer<'info>,
    #[account(mut, seeds = [MEMBER_SEED, voter.key().as_ref()], bump = member.bump)]
    pub member: Account<'info, Member>,
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    /// `init` fails if this member already voted on this proposal.
    #[account(
        init,
        payer = voter,
        space = 8 + VoteRecord::INIT_SPACE,
        seeds = [VOTE_SEED, proposal.key().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub vote_record: Account<'info, VoteRecord>,
    pub system_program: Program<'info, System>,
}

pub fn handle_cast_vote(ctx: Context<CastVote>, approve: bool) -> Result<()> {
    let proposal = &mut ctx.accounts.proposal;
    require!(
        Clock::get()?.unix_timestamp < proposal.voting_ends_at,
        ErrorCode::VotingClosed
    );

    if approve {
        proposal.yes_votes += 1;
    } else {
        proposal.no_votes += 1;
    }

    let member = &mut ctx.accounts.member;
    member.locked_until = member.locked_until.max(proposal.voting_ends_at);
    ctx.accounts.vote_record.approve = approve;
    Ok(())
}
