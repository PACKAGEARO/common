use anchor_lang::prelude::*;

/// Supported assets. Future cross-chain assets also need bridge-specific validation and custody.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum Asset {
    Sol,
    Spl { mint: Pubkey },
}

/// Spending authority the community voted to give a grantee: up to `ceiling` of `asset`
/// from `pool`, until `expires_at` or until revoked.
/// Approval reserves SOL here above rent, or SPL tokens under PDA [VAULT_SEED, grant].
#[account]
#[derive(InitSpace)]
pub struct Grant {
    /// The proposal that approved this grant.
    pub proposal: Pubkey,
    pub pool: Pubkey,
    /// Wallet allowed to withdraw. An organization can use a multisig here.
    pub grantee: Pubkey,
    pub asset: Asset,
    pub ceiling: u64,
    pub withdrawn: u64,
    pub expires_at: i64,
    pub revoked: bool,
    pub bump: u8,
}
