use solana_program::*;

use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, TokenAccount, Token}, 
    metadata::{MetadataAccount, MasterEditionAccount, Metadata, TokenRecordAccount, 
        mpl_token_metadata::{
            instructions::{TransferCpi, TransferCpiAccounts, TransferInstructionArgs}, 
            types::TokenStandard}
        },
    associated_token::AssociatedToken
};
use mpl_token_metadata::types::TransferArgs;

use crate::state::Escrow;

#[derive(Accounts)]
pub struct Make<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = maker
    )]
    pub maker_ata: Box<Account<'info, TokenAccount>>,

    pub mint_a: Box<Account<'info, Mint>>,
    pub mint_b: Box<Account<'info, Mint>>,  

    #[account(
        mut,
        seeds = [
            b"metadata",
            token_metadata_program.key().as_ref(),
            mint_a.key().as_ref()
        ],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub metadata_a: Box<Account<'info, MetadataAccount>>,
    #[account(
        seeds = [
            b"metadata",
            token_metadata_program.key().as_ref(),
            mint_a.key().as_ref(),
            b"edition",
            ],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub master_edition_a: Option<Box<Account<'info, MasterEditionAccount>>>,
    #[account(
        mut,
        seeds = [
            b"metadata",
            token_metadata_program.key().as_ref(),
            mint_a.key().as_ref(),
            b"token_record",
            maker_ata.key().as_ref(),
            ],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub maker_token_record_a: Option<Box<Account<'info, TokenRecordAccount>>>,
    #[account(mut)]
    /// CHECK: we don't need to check this
    pub vault_token_record_a: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = escrow
    )]
    pub vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = maker,
        seeds = [b"escrow", maker.key().as_ref(), mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump,
        space = Escrow::space()
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

impl<'info> Make<'info> {
    pub fn make(
        &mut self, 
        amount: u64,
    ) -> Result<()> {
        
        //All deferred errors
        let mint_a_key = self.mint_a.key();
        let metadata_a_token_standard = self.metadata_a.token_standard.as_ref().unwrap();
        let token_metadata_program_key = self.token_metadata_program.key();
        let mut master_edition_info: AccountInfo<'_>;
        let mut token_record_info: AccountInfo<'_>;
        let mut vault_token_record_info: AccountInfo<'_>;

        self.escrow.maker = *self.maker.key;
        self.escrow.mint_a = *self.mint_a.to_account_info().key;
        self.escrow.mint_b = *self.mint_b.to_account_info().key;

        // Set-up the Instruction based on the token standard
        let mut edition: Option<&AccountInfo> = None;
        let mut token_record: Option<&AccountInfo> = None;
        let mut destination_token_record: Option<&AccountInfo> = None;
        let mut transfer_args = TransferArgs::V1 {
            amount,
            authorization_data: None,
        };

        if metadata_a_token_standard == &TokenStandard::Fungible {
            self.escrow.mint_a_type = 0;
        } else if metadata_a_token_standard == &TokenStandard::NonFungible {
            master_edition_info = self.master_edition_a.as_ref().unwrap().to_account_info();
            edition = Some(&master_edition_info);
            transfer_args = TransferArgs::V1 {
                amount: 1,
                authorization_data: None,
            };
            self.escrow.mint_a_type = 1;
        } else if metadata_a_token_standard == &TokenStandard::ProgrammableNonFungible {

            //Check the token record
            let token_record_seed = [
                b"metadata",
                token_metadata_program_key.as_ref(),
                mint_a_key.as_ref(),
                b"token_record",
                self.maker_ata.to_account_info().key.as_ref(),
            ];
            let (vault_token_record_a, _bump) = Pubkey::find_program_address(&token_record_seed, &token_metadata_program_key);
            require_eq!(vault_token_record_a, self.vault_token_record_a.key());

            master_edition_info = self.master_edition_a.as_ref().unwrap().to_account_info();
            token_record_info = self.maker_token_record_a.as_ref().unwrap().to_account_info();
            vault_token_record_info = self.vault_token_record_a.to_account_info();


            edition = Some(&master_edition_info);
            token_record = Some(&token_record_info);
            destination_token_record = Some(&vault_token_record_info);
            transfer_args = TransferArgs::V1 {
                amount: 1,
                authorization_data: None,
            };
            self.escrow.mint_a_type = 2;
        };

        // Build the TransferCpi instruction to transfer the token from the maker to the escrow
        let program = &self.token_metadata_program.to_account_info();
        let token = &self.maker_ata.to_account_info();
        let token_owner = &self.maker.to_account_info();
        let destination_token = &self.vault.to_account_info();
        let destination_owner = &self.escrow.to_account_info();
        let mint = &self.mint_a.to_account_info();
        let metadata = &self.metadata_a.to_account_info();
        let authority = &self.maker.to_account_info();
        let payer = &self.maker.to_account_info();
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