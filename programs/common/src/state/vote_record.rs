use anchor_lang::prelude::*;

/// One per (proposal, member). Its existence is what stops a member voting twice.
#[account]
#[derive(InitSpace)]
pub struct VoteRecord {
    pub approve: bool,
}
