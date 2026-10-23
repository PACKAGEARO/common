use anchor_lang::prelude::*;

use crate::{constants::MAX_DESCRIPTION_LEN, error::ErrorCode, state::Asset};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ProposalAction {
    /// Let `grantee` withdraw up to `ceiling` of `asset` from `pool` until `expires_at`.
    ApproveGrant {
        pool: Pubkey,
        grantee: Pubkey,
        asset: Asset,
        ceiling: u64,
        expires_at: i64,
    },
    /// Cancel a grant's remaining spending authority.
    RevokeGrant { grant: Pubkey },
}

#[account]
#[derive(InitSpace)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Pubkey,
    #[max_len(MAX_DESCRIPTION_LEN)]
    pub description: String,
    pub action: ProposalAction,
    pub yes_votes: u32,
    pub no_votes: u32,
    /// Member-based quorum fixed when this proposal was created.
    pub quorum: u32,
    pub voting_ends_at: i64,
    pub executed: bool,
    pub bump: u8,
}

impl Proposal {
    /// Voting has ended, quorum was met, yes beat no, and it hasn't been executed.
    pub fn require_passed(&self) -> Result<()> {
        require!(!self.executed, ErrorCode::AlreadyExecuted);
        require!(
            Clock::get()?.unix_timestamp >= self.voting_ends_at,
            ErrorCode::VotingOpen
        );
        require!(
            u64::from(self.yes_votes) + u64::from(self.no_votes) >= u64::from(self.quorum),
            ErrorCode::QuorumNotReached
        );
        require!(self.yes_votes > self.no_votes, ErrorCode::ProposalRejected);
        Ok(())
    }
}
