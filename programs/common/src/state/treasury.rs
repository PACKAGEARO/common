use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Treasury {
    pub cmn_mint: Pubkey,
    /// CMN a wallet stakes to become a member. One member = one vote.
    pub min_stake: u64,
    /// Seconds a proposal stays open for voting.
    pub voting_period: i64,
    /// Percentage of members who must vote (yes + no), rounded up per proposal.
    pub quorum_percent: u8,
    pub member_count: u32,
    pub proposal_count: u64,
    pub pool_count: u64,
    pub bump: u8,
}

impl Treasury {
    /// Snapshot the participation threshold when voting opens; later joins/leaves cannot move it.
    pub fn required_votes(&self) -> u32 {
        (u64::from(self.member_count) * u64::from(self.quorum_percent))
            .div_ceil(100)
            .max(1) as u32
    }
}
