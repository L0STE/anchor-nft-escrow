use anchor_lang::prelude::*;

#[account]
pub struct Escrow {
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_a_type: u8,
    pub mint_b: Pubkey,
}

// Mint_a_type == 0: FT, NFT; 1 == pNFT

impl Escrow {
    pub fn space() -> usize {
        8 +     // Discriminator
        32 +    // maker
        32 +    // mint_a
        1 +     // mint_a_type
        32      // mint_b
    }
}