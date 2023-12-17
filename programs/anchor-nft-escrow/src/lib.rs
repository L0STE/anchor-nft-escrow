use anchor_lang::prelude::*;

pub mod errors;
pub mod state;

pub mod context;
pub use context::*;

declare_id!("2VYDrwoKRKNgvmQo3DHfcLfQFAuijjNEAY9ZoXfv8GfZ");

#[program]
pub mod nft_escrow {
    use super::*;

    pub fn make(ctx: Context<Make>, amount: u64) -> Result<()> {
        ctx.accounts.make(amount)
    }

    // pub fn take(ctx: Context<Take>) -> Result<()> {
    //     ctx.accounts.take()
    // }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        ctx.accounts.close()
    }
}
