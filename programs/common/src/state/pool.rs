use anchor_lang::prelude::*;

use crate::constants::MAX_POOL_NAME_LEN;

/// A named grant pool. Its funds sit in the vault PDA [VAULT_SEED, pool]: SOL directly,
/// SPL tokens in token accounts owned by that vault. Anyone can fund it.
#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub id: u64,
    #[max_len(MAX_POOL_NAME_LEN)]
    pub name: String,
    pub creator: Pubkey,
    pub bump: u8,
}
