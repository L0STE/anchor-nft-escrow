use anchor_lang::prelude::*;

pub mod errors;
pub mod state;

pub mod context;
pub use context::*;

declare_id!("2VYDrwoKRKNgvmQo3DHfcLfQFAuijjNEAY9ZoXfv8GfZ");

#[program]
pub mod nft_escrow {
    use super::*;

    pub fn make(ctx: Context<Make>, maker_amount: u64, taker_amount: u64) -> Result<()> {
        ctx.accounts.make(maker_amount, taker_amount)
    }

    pub fn take_from_escrow(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.take_from_escrow(ctx.bumps)
    }

    pub fn taker_to_maker(ctx: Context<Take>, amount: u64) -> Result<()> {
        ctx.accounts.taker_to_maker(amount)
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        ctx.accounts.close(ctx.bumps)
    }
}
