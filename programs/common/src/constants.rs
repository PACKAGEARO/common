use anchor_lang::prelude::*;

#[constant]
pub const TREASURY_SEED: &[u8] = b"treasury";

#[constant]
pub const POOL_SEED: &[u8] = b"pool";

/// Each pool's vault is the PDA [VAULT_SEED, pool].
#[constant]
pub const VAULT_SEED: &[u8] = b"vault";

#[constant]
pub const STAKE_VAULT_SEED: &[u8] = b"stake_vault";

#[constant]
pub const MEMBER_SEED: &[u8] = b"member";

#[constant]
pub const PROPOSAL_SEED: &[u8] = b"proposal";

#[constant]
pub const VOTE_SEED: &[u8] = b"vote";

#[constant]
pub const GRANT_SEED: &[u8] = b"grant";

pub const MAX_DESCRIPTION_LEN: usize = 200;
pub const MAX_POOL_NAME_LEN: usize = 32;
