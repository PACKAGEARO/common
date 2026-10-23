pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("12Jd4XMpddAdZ4GpnB61zxmGtwMEDKpJneg676v5HHXG");

#[program]
pub mod common {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        min_stake: u64,
        voting_period: i64,
        quorum_percent: u8,
    ) -> Result<()> {
        crate::instructions::initialize::handle_initialize(
            ctx,
            min_stake,
            voting_period,
            quorum_percent,
        )
    }

    pub fn join(ctx: Context<Join>) -> Result<()> {
        crate::instructions::join::handle_join(ctx)
    }

    pub fn leave(ctx: Context<Leave>) -> Result<()> {
        crate::instructions::leave::handle_leave(ctx)
    }

    pub fn create_pool(ctx: Context<CreatePool>, name: String) -> Result<()> {
        crate::instructions::create_pool::handle_create_pool(ctx, name)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        crate::instructions::deposit::handle_deposit(ctx, amount)
    }

    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        description: String,
        action: ProposalAction,
    ) -> Result<()> {
        crate::instructions::create_proposal::handle_create_proposal(ctx, description, action)
    }

    pub fn cast_vote(ctx: Context<CastVote>, approve: bool) -> Result<()> {
        crate::instructions::cast_vote::handle_cast_vote(ctx, approve)
    }

    pub fn execute_grant_proposal(ctx: Context<ExecuteGrantProposal>) -> Result<()> {
        crate::instructions::execute_grant_proposal::handle_execute_grant_proposal(ctx)
    }

    pub fn execute_revoke_proposal(ctx: Context<ExecuteRevokeProposal>) -> Result<()> {
        crate::instructions::execute_revoke_proposal::handle_execute_revoke_proposal(ctx)
    }

    pub fn withdraw_grant(ctx: Context<WithdrawGrant>, amount: u64) -> Result<()> {
        crate::instructions::withdraw_grant::handle_withdraw_grant(ctx, amount)
    }

    /// Return unused reserves after expiry; anyone can trigger this fixed-destination return.
    pub fn release_grant(ctx: Context<ReleaseGrant>) -> Result<()> {
        crate::instructions::release_grant::handle_release_grant(ctx)
    }
}
