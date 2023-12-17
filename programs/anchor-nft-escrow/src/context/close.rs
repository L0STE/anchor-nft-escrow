use solana_program::*;
use anchor_lang::prelude::*;
use mpl_token_metadata::types::TransferArgs;
use anchor_spl::{
    token::{Mint, TokenAccount, Token}, 
    associated_token::AssociatedToken,
    metadata::{MetadataAccount, MasterEditionAccount, Metadata, TokenRecordAccount, 
        mpl_token_metadata::instructions::{TransferCpi, TransferCpiAccounts, TransferInstructionArgs}
    }
};

use crate::state::{Escrow, MintType::*};

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    #[account(
        mut,
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
            mint_a.key().as_ref(),
            ],
        seeds::program = token_metadata_program.key(),
        bump,)]
    pub maker_token_record_a: Option<Box<Account<'info, TokenRecordAccount>>>,
    #[account(
        mut,
        seeds = [
            b"metadata",
            token_metadata_program.key().as_ref(),
            mint_a.key().as_ref(),
            b"token_record",
            vault.key().as_ref(),
            ],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub vault_token_record_a: Option<Box<Account<'info, TokenRecordAccount>>> ,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow
    )]
    pub vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        close = maker,
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

impl<'info> Close<'info> {
    pub fn close(
        &mut self, 
        bumps: CloseBumps,
    ) -> Result<()> {

        let master_edition_info: AccountInfo<'_>;
        let token_record_info: AccountInfo<'_>;
        let vault_token_record_info: AccountInfo<'_>;

        // Set-up the Instruction based on the token standard
        let mut edition: Option<&AccountInfo> = None;
        let mut token_record: Option<&AccountInfo> = None;
        let mut destination_token_record: Option<&AccountInfo> = None;
        let mut transfer_args = TransferArgs::V1 {
            amount: self.vault.amount,
            authorization_data: None,
        };

        match self.escrow.mint_a_type {
            Fungible => {},
            NonFungible => {
                master_edition_info = self.master_edition_a.as_ref().unwrap().to_account_info();
                edition = Some(&master_edition_info);
                transfer_args = TransferArgs::V1 {
                    amount: 1,
                    authorization_data: None,
                };
            },
            ProgrammableNonFungible => {
                master_edition_info = self.master_edition_a.as_ref().unwrap().to_account_info();
                token_record_info = self.maker_token_record_a.as_ref().unwrap().to_account_info();
                vault_token_record_info = self.vault_token_record_a.as_ref().unwrap().to_account_info();

                edition = Some(&master_edition_info);
                token_record = Some(&vault_token_record_info);
                destination_token_record = Some(&token_record_info);
                transfer_args = TransferArgs::V1 {
                    amount: 1,
                    authorization_data: None,
                };
            }
            
        }

        // Build the TransferCpi instruction to transfer the token from the maker to the escrow
        let program = &self.token_metadata_program.to_account_info();
        let token = &self.vault.to_account_info();
        let token_owner = &self.escrow.to_account_info();
        let destination_token = &self.maker_ata.to_account_info();
        let destination_owner = &self.maker.to_account_info();
        let mint = &self.mint_a.to_account_info();
        let metadata = &self.metadata_a.to_account_info();
        let authority = &self.escrow.to_account_info();
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

        let mint_a_key = self.mint_a.key();
        let mint_b_key = self.mint_b.key();
        let maker_key = self.maker.key();

        let seeds = &[
            "escrow".as_bytes(),
            maker_key.as_ref(),
            mint_a_key.as_ref(),
            mint_b_key.as_ref(),
            &[bumps.escrow]
        ];
        let signer_seeds = &[&seeds[..]];

        transfer_cpi.invoke_signed(signer_seeds)?;

        Ok(())
    }
}