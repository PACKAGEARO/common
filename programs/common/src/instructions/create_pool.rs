use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::ErrorCode,
    state::{Member, Pool, Treasury},
};

#[derive(Accounts)]
pub struct CreatePool<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(mut, seeds = [TREASURY_SEED], bump = treasury.bump)]
    pub treasury: Account<'info, Treasury>,
    /// Only members can open pools. Spending from a pool still takes a vote.
    #[account(seeds = [MEMBER_SEED, creator.key().as_ref()], bump = member.bump)]
    pub member: Account<'info, Member>,
    #[account(
        init,
        payer = creator,
        space = 8 + Pool::INIT_SPACE,
        seeds = [POOL_SEED, treasury.pool_count.to_le_bytes().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,
    pub system_program: Program<'info, System>,
}

pub fn handle_create_pool(ctx: Context<CreatePool>, name: String) -> Result<()> {
    require!(name.len() <= MAX_POOL_NAME_LEN, ErrorCode::NameTooLong);

    let treasury = &mut ctx.accounts.treasury;
    ctx.accounts.pool.set_inner(Pool {
        id: treasury.pool_count,
        name,
        creator: ctx.accounts.creator.key(),
        bump: ctx.bumps.pool,
    });
    treasury.pool_count += 1;
    Ok(())
}
