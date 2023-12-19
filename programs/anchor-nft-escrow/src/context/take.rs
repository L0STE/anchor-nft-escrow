use solana_program::*;

use anchor_lang::{prelude::*, Discriminator};
use anchor_spl::{
    token::{Mint, TokenAccount, Token}, 
    metadata::{MetadataAccount, MasterEditionAccount, Metadata, TokenRecordAccount, 
        mpl_token_metadata::{
            instructions::{TransferCpi, TransferCpiAccounts, TransferInstructionArgs}, 
            types::TokenStandard}
        },
    associated_token::{AssociatedToken, get_associated_token_address}
};
use mpl_token_metadata::types::TransferArgs;
use sysvar::instructions::{load_current_index_checked, load_instruction_at_checked};

use crate::{
    state::Escrow,
    errors::{EscrowError, IntrospectionError},
};

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub maker: SystemAccount<'info>,
    #[account(mut)]
    pub taker: Signer<'info>,

    pub mint_a: Box<Account<'info, Mint>>, 
    pub mint_b: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub metadata: Box<Account<'info, MetadataAccount>>,
    #[account(mut)]
    pub master_edition: Option<Box<Account<'info, MasterEditionAccount>>>,
    #[account(mut)]
    pub origin_token_record: Option<Box<Account<'info, TokenRecordAccount>>>,
    #[account(mut)]
    /// CHECK: we're checking this later
    pub destination_token_record: UncheckedAccount<'info>,

    #[account(mut)]
    pub origin_ata: Box<Account<'info, TokenAccount>>, //Start: Vault; End: Taker_ata
    #[account(mut)]
    pub destination_ata: Box<Account<'info, TokenAccount>>, //Start: Taker_ata; End: Maker_ata

    #[account(
        seeds = [b"escrow", maker.key().as_ref(), mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(address = sysvar::instructions::id())]
    /// CHECK: we don't need to check this
    pub sysvar_instructions: UncheckedAccount<'info>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>
}

impl<'info> Take<'info> {
    pub fn take_from_escrow(
        &mut self,
        bumps: TakeBumps,
    ) -> Result<()> {

        //All deferred errors
        let mint_a_key = self.mint_a.key();
        let mint_b_key = self.mint_b.key();
        let maker_key = self.maker.key();
        let metadata_token_standard = self.metadata.token_standard.as_ref().unwrap();
        let token_metadata_program_key = self.token_metadata_program.key();
        let master_edition_info: AccountInfo<'_>;
        let token_record_info: AccountInfo<'_>;
        let destination_token_record_info: AccountInfo<'_>;

        // Set-up the Instruction based on the token standard - We set it up as a Fungible by default
        let mut edition: Option<&AccountInfo> = None;
        let mut token_record: Option<&AccountInfo> = None;
        let mut destination_token_record: Option<&AccountInfo> = None;
        let mut transfer_args = TransferArgs::V1 {
            amount: self.origin_ata.amount,
            authorization_data: None,
        };

        // Check the Metadata account
        let metadata_seed = [
            b"metadata",
            token_metadata_program_key.as_ref(),
            mint_a_key.as_ref(),
        ];
        let (metadata, _) = Pubkey::find_program_address(&metadata_seed, &self.token_metadata_program.key());
        require_keys_eq!(metadata, self.metadata.key(), EscrowError::MetadataAccountDoesNotMatch);


        if metadata_token_standard == &TokenStandard::NonFungible || metadata_token_standard == &TokenStandard::ProgrammableNonFungible {
            
            //Check the Master Edition account
            let master_edition_seed = [
                b"metadata",
                token_metadata_program_key.as_ref(),
                mint_a_key.as_ref(),
                b"edition",
            ];
            let (master_edition, _) = Pubkey::find_program_address(&master_edition_seed, &self.token_metadata_program.key());
            require_keys_eq!(master_edition, self.master_edition.as_ref().unwrap().key(), EscrowError::MasterEditionAccountDoesNotMatch);
            
            master_edition_info = self.master_edition.as_ref().unwrap().to_account_info();
            edition = Some(&master_edition_info);
            transfer_args = TransferArgs::V1 {
                amount: 1,
                authorization_data: None,
            };
        } 
        
        if metadata_token_standard == &TokenStandard::ProgrammableNonFungible {

            //Check the token record
            let token_record_account_seed = [
                b"metadata",
                token_metadata_program_key.as_ref(),
                mint_a_key.as_ref(),
                b"token_record",
                self.origin_ata.to_account_info().key.as_ref(),
            ];
            let (origin_token_record_pda, _) = Pubkey::find_program_address(&token_record_account_seed, &self.token_metadata_program.key());
            require_keys_eq!(origin_token_record_pda, self.origin_token_record.as_ref().unwrap().key(), EscrowError::TokenRecordAccountDoesNotMatch);

            let token_record_account_seed = [
                b"metadata",
                token_metadata_program_key.as_ref(),
                mint_a_key.as_ref(),
                b"token_record",
                self.destination_ata.to_account_info().key.as_ref(),
            ];
            let (destination_token_record_pda, _) = Pubkey::find_program_address(&token_record_account_seed, &self.token_metadata_program.key());
            require_keys_eq!(destination_token_record_pda, self.destination_token_record.key(), EscrowError::TokenRecordAccountDoesNotMatch);

            token_record_info = self.origin_token_record.as_ref().unwrap().to_account_info();
            destination_token_record_info = self.destination_token_record.to_account_info();

            token_record = Some(&token_record_info);
            destination_token_record = Some(&destination_token_record_info);
        };

        // Build the TransferCpi instruction to transfer the token from the maker to the escrow
        let program = &self.token_metadata_program.to_account_info();
        let token = &self.origin_ata.to_account_info();
        let token_owner = &self.escrow.to_account_info();
        let destination_token = &self.destination_ata.to_account_info();
        let destination_owner = &self.taker.to_account_info();
        let mint = &self.mint_a.to_account_info();
        let metadata = &self.metadata.to_account_info();
        let authority = &self.escrow.to_account_info();
        let payer = &self.taker.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let sysvar_instructions = &self.sysvar_instructions.to_account_info();
        let spl_token_program = &self.token_program.to_account_info();
        let spl_ata_program = &self.associated_token_program.to_account_info();
        //TODO After
        let authorization_rules_program = None;
        let authorization_rules = None;

        let transfer_cpi = TransferCpi::new(
            program,
            TransferCpiAccounts {
                token,
                token_owner,
                destination_token,
                destination_owner,
                mint,
                metadata,
                edition,
                token_record,
                destination_token_record,
                authority,
                payer,
                system_program,
                sysvar_instructions,
                spl_token_program,
                spl_ata_program,
                authorization_rules_program,
                authorization_rules,              
            },
            TransferInstructionArgs {
                transfer_args,
            },
        );

        let seeds = &[
            "escrow".as_bytes(),
            maker_key.as_ref(),
            mint_a_key.as_ref(),
            mint_b_key.as_ref(),
            &[bumps.escrow]
        ];
        let signer_seeds = &[&seeds[..]];

        transfer_cpi.invoke_signed(signer_seeds)?;

        // Set up Instruction Introspection to make sure that:
        // 1. The token was transferred from the taker to the maker after this transaction
        // 2. It happened atomically

        // We load the current transaction, see the current index and see if the instruction after this is the trasnfer_to_escrow instruction
        let index = load_current_index_checked(&self.sysvar_instructions.to_account_info())?;
        let ix = load_instruction_at_checked(index as usize + 1, &self.sysvar_instructions.to_account_info())?;


        // We need to make sure that: 
        // 1. The instruction is is the trasnfer_to_escrow instruction.
        // 2. The Token is the same as the one specified in the escrow && the receiver is the maker.
        // 3. The amount sent is the same of the amount specified in the escrow.

        // Testing 1: We know that the first 8 bytes of the data of the transaction is the discriminator
        require_keys_eq!(ix.program_id, crate::ID, IntrospectionError::InvalidProgram);
        require!(ix.data[0..8].eq(crate::instruction::TakerToMaker::DISCRIMINATOR.as_slice()), IntrospectionError::InvalidDiscriminator);

        // Testing 2: We know that the 10th account is going to be the destination_ata, and we know that we want that as the maker_ata
        let maker_ata = get_associated_token_address(&self.maker.key(), &self.mint_b.key());
        require_keys_eq!(ix.accounts.get(9).unwrap().pubkey, maker_ata, IntrospectionError::InvalidMakerATA);
        
        // Testing 3: We know that 8 bites (u8) after the discriminator will be the amount since it's the only variable we are passing
        require!(ix.data[8..16].eq(&self.escrow.mint_b_amount.to_le_bytes()), IntrospectionError::InvalidAmount);

        Ok(())
    }

    pub fn taker_to_maker(
        &mut self,
        amount: u64,
    ) -> Result<()> {

        require_eq!(amount, self.escrow.mint_b_amount, EscrowError::InvalidAmount);

        //All deferred errors
        let mint_b_key = self.mint_b.key();
        let metadata_token_standard = self.metadata.token_standard.as_ref().unwrap();
        let token_metadata_program_key = self.token_metadata_program.key();
        let master_edition_info: AccountInfo<'_>;
        let token_record_info: AccountInfo<'_>;
        let destination_token_record_info: AccountInfo<'_>;

        // Set-up the Instruction based on the token standard - We set it up as a Fungible by default
        let mut edition: Option<&AccountInfo> = None;
        let mut token_record: Option<&AccountInfo> = None;
        let mut destination_token_record: Option<&AccountInfo> = None;
        let transfer_args = TransferArgs::V1 {
            amount,
            authorization_data: None,
        };

        // Check the Metadata account
        let metadata_seed = [
            b"metadata",
            token_metadata_program_key.as_ref(),
            mint_b_key.as_ref(),
        ];
        let (metadata, _) = Pubkey::find_program_address(&metadata_seed, &self.token_metadata_program.key());
        require_keys_eq!(metadata, self.metadata.key(), EscrowError::MetadataAccountDoesNotMatch);


        if metadata_token_standard == &TokenStandard::NonFungible || metadata_token_standard == &TokenStandard::ProgrammableNonFungible {
            
            //Check the Master Edition account
            let master_edition_seed = [
                b"metadata",
                token_metadata_program_key.as_ref(),
                mint_b_key.as_ref(),
                b"edition",
            ];
            let (master_edition, _) = Pubkey::find_program_address(&master_edition_seed, &self.token_metadata_program.key());
            require_keys_eq!(master_edition, self.master_edition.as_ref().unwrap().key(), EscrowError::MasterEditionAccountDoesNotMatch);
            
            master_edition_info = self.master_edition.as_ref().unwrap().to_account_info();
            edition = Some(&master_edition_info);
        } 
        
        if metadata_token_standard == &TokenStandard::ProgrammableNonFungible {

            //Check the token record
            let token_record_account_seed = [
                b"metadata",
                token_metadata_program_key.as_ref(),
                mint_b_key.as_ref(),
                b"token_record",
                self.origin_ata.to_account_info().key.as_ref(),
            ];
            let (origin_token_record_pda, _) = Pubkey::find_program_address(&token_record_account_seed, &self.token_metadata_program.key());
            require_keys_eq!(origin_token_record_pda, self.origin_token_record.as_ref().unwrap().key(), EscrowError::TokenRecordAccountDoesNotMatch);

            let token_record_account_seed = [
                b"metadata",
                token_metadata_program_key.as_ref(),
                mint_b_key.as_ref(),
                b"token_record",
                self.destination_ata.to_account_info().key.as_ref(),
            ];
            let (destination_token_record_pda, _) = Pubkey::find_program_address(&token_record_account_seed, &self.token_metadata_program.key());
            require_keys_eq!(destination_token_record_pda, self.destination_token_record.key(), EscrowError::TokenRecordAccountDoesNotMatch);

            token_record_info = self.origin_token_record.as_ref().unwrap().to_account_info();
            destination_token_record_info = self.destination_token_record.to_account_info();

            token_record = Some(&token_record_info);
            destination_token_record = Some(&destination_token_record_info);
        };

        // Build the TransferCpi instruction to transfer the token from the maker to the escrow
        let program = &self.token_metadata_program.to_account_info();
        let token = &self.origin_ata.to_account_info();
        let token_owner = &self.taker.to_account_info();
        let destination_token = &self.destination_ata.to_account_info();
        let destination_owner = &self.maker.to_account_info();
        let mint = &self.mint_b.to_account_info();
        let metadata = &self.metadata.to_account_info();
        let authority = &self.taker.to_account_info();
        let payer = &self.taker.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let sysvar_instructions = &self.sysvar_instructions.to_account_info();
        let spl_token_program = &self.token_program.to_account_info();
        let spl_ata_program = &self.associated_token_program.to_account_info();
        //TODO After
        let authorization_rules_program = None;
        let authorization_rules = None;

        let transfer_cpi = TransferCpi::new(
            program,
            TransferCpiAccounts {
                token,
                token_owner,
                destination_token,
                destination_owner,
                mint,
                metadata,
                edition,
                token_record,
                destination_token_record,
                authority,
                payer,
                system_program,
                sysvar_instructions,
                spl_token_program,
                spl_ata_program,
                authorization_rules_program,
                authorization_rules,              
            },
            TransferInstructionArgs {
                transfer_args,
            },
        );

        transfer_cpi.invoke()?;

        Ok(())
    }

}