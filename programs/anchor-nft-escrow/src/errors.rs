use anchor_lang::error_code;

#[error_code]
pub enum EscrowError {
    #[msg("The Metadata Account doesn't match.")]
    MetadataAccountDoesNotMatch,
    #[msg("The Master Edition Account doesn't match.")]
    MasterEditionAccountDoesNotMatch,
    #[msg("The Token Record Account doesn't match.")]
    TokenRecordAccountDoesNotMatch,
    #[msg("The Amount of Token sent to the Maker doesn't match.")]
    InvalidAmount
}

#[error_code]
pub enum IntrospectionError {
    #[msg("Invalid discriminator")]
    InvalidDiscriminator,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid program")]
    InvalidProgram,
    #[msg("Invalid Maker ATA")]
    InvalidMakerATA
}