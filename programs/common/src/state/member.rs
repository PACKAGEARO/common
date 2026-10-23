use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Member {
    pub owner: Pubkey,
    /// Stake can't be withdrawn until every proposal this member voted on has closed,
    /// so the same CMN can't vote twice by moving to another wallet.
    pub locked_until: i64,
    pub bump: u8,
}
