use anchor_lang::prelude::*;

// [DEPENDENCY] Imports ReleaseGrant and release_funds from programs/common/src/instructions/release_grant.rs
use super::release_grant::*;

use crate::{
    error::ErrorCode,
    state::{Proposal, ProposalAction},
};

#[event]
pub struct GrantRevoked {
    pub grant: Pubkey,
    pub withdrawn: u64,
}

/// Anyone can execute a passed RevokeGrant proposal.
#[derive(Accounts)]
pub struct ExecuteRevokeProposal<'info> {
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    pub release: ReleaseGrant<'info>,
}

pub fn handle_execute_revoke_proposal(ctx: Context<ExecuteRevokeProposal>) -> Result<()> {
    let proposal = &ctx.accounts.proposal;
    proposal.require_passed()?;
    let ProposalAction::RevokeGrant { grant } = proposal.action else {
        return err!(ErrorCode::WrongProposalType);
    };
    require_keys_eq!(
        ctx.accounts.release.grant.key(),
        grant,
        ErrorCode::WrongGrant
    );

    ctx.accounts.release.grant.revoked = true;
    release_funds(&ctx.accounts.release, ctx.bumps.release.vault)?;
    ctx.accounts.proposal.executed = true;

    emit!(GrantRevoked {
        grant,
        withdrawn: ctx.accounts.release.grant.withdrawn,
    });
    Ok(())
}
