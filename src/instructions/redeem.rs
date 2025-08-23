use core::mem::size_of;
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::find_program_address,
    ProgramResult,
};
use pinocchio_token::instructions::Burn;

// ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
pub const ASSOCIATED_TOKEN_PROGRAM_ID: [u8; 32] = [
    140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19, 153, 218,
    255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89,
];

pub struct Redeem<'a> {
    pub accounts: RedeemAccounts<'a>,
    pub instruction_data: RedeemInstructionData,
}

pub struct RedeemAccounts<'a> {
    pub redeemer: &'a AccountInfo, //signer
    pub mint: &'a AccountInfo, //@audit-issue : if left unvalidated, users can burn fake tokens and claim real tokens
    pub mint_ata: &'a AccountInfo, // depositors mint ATA
    pub global_pda: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

pub struct RedeemInstructionData {
    pub amount: u64,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for Redeem<'a> {
    type Error = ProgramError;

    fn try_from((_accounts, data): (&'a [AccountInfo], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = RedeemAccounts::try_from(_accounts)?;
        let instruction_data = RedeemInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for RedeemAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [redeemer, mint, mint_ata, global_pda, token_program, system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // dev: Skip signer and token authority checks here, as the Burn instruction already enforces them.

        // dev: Must validate the mint to prevent users from claiming SOL by burning arbitrary tokens they control.
        let (mint_key, _) = find_program_address(&[b"mint"], &crate::ID);

        if mint.key().ne(&mint_key) {
            return Err(ProgramError::InvalidSeeds); // dev: add custom error later
        };

        // dev: Adding checks for the user's ATA and global PDA mainly helps guide correct account inputs; omitting them does not pose a direct security risk.

        let (ata_key, _) = find_program_address(
            &[
                redeemer.key().as_ref(),      // owner
                token_program.key().as_ref(), // token_program
                mint.key().as_ref(),          // mint
            ],
            &ASSOCIATED_TOKEN_PROGRAM_ID.into(), // associated_token_program
        );

        let (global_key, _) = find_program_address(&[b"global"], &crate::ID);

        if mint_ata.key().ne(&ata_key) {
            return Err(ProgramError::InvalidSeeds); // dev : later use custom : `OnlyATAAccepted`
        };

        if global_pda.key().ne(&global_key) {
            return Err(ProgramError::InvalidSeeds);
        };

        // return accounts
        Ok(Self {
            redeemer,
            mint,
            mint_ata,
            global_pda,
            token_program,
            system_program,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for RedeemInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len().ne(&size_of::<u64>()) {
            return Err(ProgramError::InvalidInstructionData);
        };

        let amount = u64::from_le_bytes(data.try_into().unwrap());

        if amount.eq(&0) {
            return Err(ProgramError::InvalidInstructionData);
        }; // dev : use custom error for 0 amounts

        Ok(Self { amount })
    }
}

///// logic
impl<'a> Redeem<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2;

    // Architecture :
    // --> burn amount of shares from user ata
    // --> send sol from global_pda to user

    pub fn process(&mut self) -> ProgramResult {
        let amount = self.instruction_data.amount;

        // burn given amount of shares
        Burn {
            account: self.accounts.mint_ata,
            mint: self.accounts.mint,
            authority: self.accounts.redeemer,
            amount,
        }
        .invoke()?;

        // transfer 1:1 sol amounts for burnt amounts
        let global_lamports_initial = self.accounts.global_pda.lamports();
        let redeemer_lamports_initial = self.accounts.redeemer.lamports();

        // dev : Invariant: global_pda will always be rent exempt, because it is impossible to mint shares without having equivalent sol deposits, therefore neglect rent exemption checks before withdraw

        unsafe {
            *self.accounts.global_pda.borrow_mut_lamports_unchecked() = global_lamports_initial
                .checked_sub(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?; // dev : later use custom Error `ArthmeticUnderflow`
        };

        unsafe {
            *self.accounts.redeemer.borrow_mut_lamports_unchecked() = redeemer_lamports_initial
                .checked_add(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
        }

        Ok(())
    }
}
