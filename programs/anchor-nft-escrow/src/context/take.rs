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

use crate::state::{Escrow, MintType::*};

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,
    #[account(mut)]
    pub maker: SystemAccount<'info>,

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
            vault.key().as_ref(),
            ],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub vault_token_record_a: Option<Box<Account<'info, TokenRecordAccount>>>,
    #[account(mut)]
    /// CHECK: we don't need to check this
    pub taker_token_record_a: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [
            b"metadata",
            token_metadata_program.key().as_ref(),
            mint_b.key().as_ref()
        ],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub metadata_b: Box<Account<'info, MetadataAccount>>,
    #[account(
        seeds = [
            b"metadata",
            token_metadata_program.key().as_ref(),
            mint_b.key().as_ref(),
            b"edition",
            ],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub master_edition_b: Option<Box<Account<'info, MasterEditionAccount>>>,
    #[account(
        mut,
        seeds = [
            b"metadata",
            token_metadata_program.key().as_ref(),
            mint_a.key().as_ref(),
            b"token_record",
            taker_ata_b.key().as_ref(),
            ],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub taker_token_record_b: Option<Box<Account<'info, TokenRecordAccount>>>,
    #[account(mut)]
    /// CHECK: we don't need to check this
    pub maker_token_record_b: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = escrow
    )]
    pub vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker
    )]
    pub taker_ata_a: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = taker
    )]
    pub taker_ata_b: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker
    )]
    pub maker_ata_b: Box<Account<'info, TokenAccount>>,

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

impl<'info> Take<'info> {
    pub fn take(
        &mut self,
        amount: u64, //To set in the make
        bumps: TakeBumps,
    ) -> Result<()> {
        
        /*  Transfer from the Vault to the Taker  */

        //All deferred errors
        let mint_a_key = self.mint_a.key();
        let token_metadata_program_key = self.token_metadata_program.key();
        let mut master_edition_info: AccountInfo<'_>;
        let mut token_record_info: AccountInfo<'_>;
        let mut destination_token_record_info: AccountInfo<'_>;

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

                //Check the token record
                let token_record_seed = [
                    b"metadata",
                    token_metadata_program_key.as_ref(),
                    mint_a_key.as_ref(),
                    b"token_record",
                    self.taker_ata_a.to_account_info().key.as_ref(),
                ];
                let (taker_token_record_a, _bump) = Pubkey::find_program_address(&token_record_seed, &token_metadata_program_key);
                require_eq!(taker_token_record_a, self.taker_token_record_a.key());

                master_edition_info = self.master_edition_a.as_ref().unwrap().to_account_info();
                token_record_info = self.vault_token_record_a.as_ref().unwrap().to_account_info();
                destination_token_record_info = self.taker_token_record_a.as_ref().to_account_info();

                edition = Some(&master_edition_info);
                token_record = Some(&token_record_info);
                destination_token_record = Some(&destination_token_record_info);
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
        let destination_token = &self.taker_ata_a.to_account_info();
        let destination_owner = &self.taker.to_account_info();
        let mint = &self.mint_a.to_account_info();
        let metadata = &self.metadata_a.to_account_info();
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


        /*  Transfer from the Taker to the Maker  */

        //All deferred errors
        let mint_b_key = self.mint_b.key();
        let metadata_b_token_standard = self.metadata_b.token_standard.as_ref().unwrap();

        // Set-up the Instruction based on the token standard
        edition = None;
        token_record = None;
        destination_token_record = None;
        transfer_args = TransferArgs::V1 {
            amount,
            authorization_data: None,
        };

        if metadata_b_token_standard == &TokenStandard::NonFungible {
            master_edition_info = self.master_edition_b.as_ref().unwrap().to_account_info();
            edition = Some(&master_edition_info);
            transfer_args = TransferArgs::V1 {
                amount: 1,
                authorization_data: None,
            };
        } else if metadata_b_token_standard == &TokenStandard::ProgrammableNonFungible {

            //Check the token record
            let token_record_seed = [
                b"metadata",
                token_metadata_program_key.as_ref(),
                mint_b_key.as_ref(),
                b"token_record",
                self.maker_ata_b.to_account_info().key.as_ref(),
            ];
            let (maker_token_record_b, _bump) = Pubkey::find_program_address(&token_record_seed, &token_metadata_program_key);
            require_eq!(maker_token_record_b, self.maker_token_record_b.key());

            master_edition_info = self.master_edition_b.as_ref().unwrap().to_account_info();
            token_record_info = self.taker_token_record_b.as_ref().unwrap().to_account_info();
            destination_token_record_info = self.maker_token_record_b.as_ref().to_account_info();

            edition = Some(&master_edition_info);
            token_record = Some(&token_record_info);
            destination_token_record = Some(&destination_token_record_info);
            transfer_args = TransferArgs::V1 {
                amount: 1,
                authorization_data: None,
            };
        };

        // Build the TransferCpi instruction to transfer the token from the maker to the escrow
        let program = &self.token_metadata_program.to_account_info();
        let token = &self.taker_ata_b.to_account_info();
        let token_owner = &self.taker.to_account_info();
        let destination_token = &self.maker_ata_b.to_account_info();
        let destination_owner = &self.maker.to_account_info();
        let mint = &self.mint_b.to_account_info();
        let metadata = &self.metadata_b.to_account_info();
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