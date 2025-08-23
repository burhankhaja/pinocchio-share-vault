use core::mem::size_of;

use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};

use pinocchio_system::instructions::{CreateAccount, Transfer};
use pinocchio_token::{
    instructions::{InitializeAccount3, MintTo},
    state::TokenAccount,
};

// ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
pub const ASSOCIATED_TOKEN_PROGRAM_ID: [u8; 32] = [
    140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19, 153, 218,
    255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89,
];

pub struct Deposit<'a> {
    pub accounts: DepositAccounts<'a>,
    pub instruction_data: DepositInstructionData,
}

pub struct DepositAccounts<'a> {
    pub depositor: &'a AccountInfo,
    pub mint: &'a AccountInfo,
    pub mint_ata: &'a AccountInfo, // depositors mint ATA
    pub global_pda: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub associated_token_program: &'a AccountInfo,
    pub global_pda_bump: [u8; 1], // note : later correctly reset during TryFrom bump calculations
}

pub struct DepositInstructionData {
    pub amount: u64,
    // pub amount: [u8; 8], // dev : try this approach later during testing
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for Deposit<'a> {
    type Error = ProgramError;
    fn try_from((_accounts, data): (&'a [AccountInfo], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = DepositAccounts::try_from(_accounts)?;
        let instruction_data = DepositInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for DepositAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [depositor, mint, mint_ata, global_pda, token_program, system_program, associated_token_program, _] =
            accounts
        else {
            // dev : `, _` : ignore provided pump --> will be recalculated
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // dev : since runtime will make signer checks on Sol Transfer, there is no need to add them here

        // validate global_pda

        let (global_key, global_bump) = find_program_address(&[b"global"], &crate::ID);

        if global_pda.key().ne(&global_key) {
            return Err(ProgramError::InvalidSeeds);
        };

        // validate mint

        let (mint_key, _) = find_program_address(&[b"mint"], &crate::ID);

        if mint.key().ne(&mint_key) {
            return Err(ProgramError::InvalidSeeds);
        };

        // validate ata
        let (ata_key, _) = find_program_address(
            &[
                depositor.key().as_ref(),     // owner
                token_program.key().as_ref(), // token_program
                mint.key().as_ref(),          // mint
            ],
            &ASSOCIATED_TOKEN_PROGRAM_ID.into(), // associated_token_program
        );

        if mint_ata.key().ne(&ata_key) {
            return Err(ProgramError::InvalidSeeds); // dev : later use custom : `OnlyATAAccepted`
        }

        // create ata for user to store share tokens if not already created
        if mint_ata.data_is_empty() {
            // calculate rent and space
            let ata_space = TokenAccount::LEN;
            let ata_rent = Rent::get()?.minimum_balance(ata_space);

            // createAccount
            CreateAccount {
                from: depositor,
                to: mint_ata,
                lamports: ata_rent,
                space: ata_space as u64,
                owner: token_program.key(),
            }
            .invoke()?;

            // Initialize token account
            InitializeAccount3 {
                account: mint_ata,
                mint: mint,
                owner: depositor.key(),
            }
            .invoke()?;
        }

        Ok(Self {
            depositor,
            mint,
            mint_ata,
            global_pda,
            token_program,
            system_program,
            associated_token_program,
            global_pda_bump: [global_bump],
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for DepositInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len().ne(&size_of::<u64>()) {
            // personal note : using () after <u64> triggers actual func call
            return Err(ProgramError::InvalidInstructionData);
        }

        let amount = u64::from_le_bytes(data.try_into().unwrap());

        // prevent unneccessary calls with 0 amount
        if amount.eq(&0) {
            return Err(ProgramError::InvalidInstructionData);
        } // dev : later use custom errors for 0 amount

        Ok(Self { amount })
    }
}

impl<'a> Deposit<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1;

    pub fn process(&mut self) -> ProgramResult {
        let amount = self.instruction_data.amount;

        //// transfer given amount of sol to global pda
        Transfer {
            from: self.accounts.depositor,
            to: self.accounts.global_pda,
            lamports: amount,
            // lamports: u64::from_le_bytes(amount), // dev : if (DepositInstructionData.deposit : [u8; 8] )
        }
        .invoke()?;

        //// mint 1:1 shares for user deposits

        let seeds = [
            Seed::from(b"global"),
            Seed::from(&self.accounts.global_pda_bump),
        ];

        let mint_signer = Signer::from(&seeds);

        MintTo {
            mint: self.accounts.mint,
            account: self.accounts.mint_ata,
            mint_authority: self.accounts.global_pda,
            amount,
        }
        .invoke_signed(&[mint_signer])
    }
}
