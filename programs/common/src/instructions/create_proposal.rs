use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::ErrorCode,
    state::{Member, Proposal, ProposalAction, Treasury},
};

#[derive(Accounts)]
pub struct CreateProposal<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,
    #[account(mut, seeds = [TREASURY_SEED], bump = treasury.bump)]
    pub treasury: Account<'info, Treasury>,
    /// Only members can propose.
    #[account(seeds = [MEMBER_SEED, proposer.key().as_ref()], bump = member.bump)]
    pub member: Account<'info, Member>,
    #[account(
        init,
        payer = proposer,
        space = 8 + Proposal::INIT_SPACE,
        seeds = [PROPOSAL_SEED, treasury.proposal_count.to_le_bytes().as_ref()],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
    pub system_program: Program<'info, System>,
}

/// Open voting with a fixed, rounded-up participation threshold based on current membership.
pub fn handle_create_proposal(
    ctx: Context<CreateProposal>,
    description: String,
    action: ProposalAction,
) -> Result<()> {
    require!(
        description.len() <= MAX_DESCRIPTION_LEN,
        ErrorCode::DescriptionTooLong
    );

    let treasury = &mut ctx.accounts.treasury;
    let voting_ends_at = Clock::get()?.unix_timestamp + treasury.voting_period;
    if let ProposalAction::ApproveGrant {
        ceiling,
        expires_at,
        ..
    } = action
    {
        require!(
            ceiling > 0 && expires_at > voting_ends_at,
            ErrorCode::InvalidGrant
        );
    }

    ctx.accounts.proposal.set_inner(Proposal {
        id: treasury.proposal_count,
        proposer: ctx.accounts.proposer.key(),
        description,
        action,
        yes_votes: 0,
        no_votes: 0,
        quorum: treasury.required_votes(),
        voting_ends_at,
        executed: false,
        bump: ctx.bumps.proposal,
    });
    treasury.proposal_count += 1;
    Ok(())
}
