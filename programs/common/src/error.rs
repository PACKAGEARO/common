use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("This signer isn't allowed to do that")]
    Unauthorized,
    #[msg("Stake and voting period must be positive; quorum percentage must be 1 through 100")]
    InvalidConfig,
    #[msg("Description is longer than 200 bytes")]
    DescriptionTooLong,
    #[msg("Pool name is longer than 32 bytes")]
    NameTooLong,
    #[msg("Grant ceiling must be above zero and it must expire after voting ends")]
    InvalidGrant,
    #[msg("Voting on this proposal has closed")]
    VotingClosed,
    #[msg("Voting on this proposal is still open")]
    VotingOpen,
    #[msg("Proposal has already been executed")]
    AlreadyExecuted,
    #[msg("Not enough votes were cast to reach quorum")]
    QuorumNotReached,
    #[msg("Proposal did not get more yes votes than no votes")]
    ProposalRejected,
    #[msg("This instruction can't execute that kind of proposal")]
    WrongProposalType,
    #[msg("Pool does not match the proposal or grant")]
    WrongPool,
    #[msg("Grant does not match the proposal")]
    WrongGrant,
    #[msg("Token account is not owned by this pool's vault")]
    WrongVault,
    #[msg("Vault token account mint does not match the grant")]
    WrongMint,
    #[msg("SPL transfers need the token accounts for that mint")]
    MissingTokenAccount,
    #[msg("Stake is locked until every proposal you voted on has closed")]
    StakeLocked,
    #[msg("Grant has been revoked")]
    GrantRevoked,
    #[msg("Grant has expired")]
    GrantExpired,
    #[msg("Withdrawal would go over the grant's ceiling")]
    CeilingExceeded,
    #[msg("The source vault does not have enough available funds")]
    InsufficientFunds,
    #[msg("Unused funds can only return after the grant expires or is revoked")]
    GrantActive,
}
