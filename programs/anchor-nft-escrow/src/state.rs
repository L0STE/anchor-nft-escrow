use anchor_lang::prelude::*;

#[account]
pub struct Escrow {
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub mint_b_amount: u64,
}

impl Escrow {
    pub fn space() -> usize {
        8 +     // Discriminator
        32 +    // mint_a
        32 +    // mint_b
        8       // mint_b_amount
    }
}