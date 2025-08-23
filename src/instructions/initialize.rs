use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::{find_program_address, Pubkey},
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{instructions::InitializeMint2, state::Mint};

pub struct Initialize<'a> {
    pub accounts: InitializeAccounts<'a>,
}

pub struct InitializeAccounts<'a> {
    pub initializer: &'a AccountInfo,
    pub global_pda: &'a AccountInfo, // stores lamports and mints shares
    pub mint: &'a AccountInfo,       // share token
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for Initialize<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let accounts = InitializeAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for InitializeAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [initializer, global_pda, mint, system_program, token_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        let (global_pda_key, _bump) = find_program_address(&[b"global"], &crate::ID);

        if global_pda.key() != &global_pda_key {
            return Err(ProgramError::InvalidSeeds); // dev : later create custom error for this one : InvalidPda ?
        }

        let (mint_key, _) = find_program_address(&[b"mint"], &crate::ID);

        if mint.key().ne(&mint_key) {
            return Err(ProgramError::InvalidSeeds);
        }

        // dev : for now ignore other checks, note that createAccount will return Error on already created accounts ?

        Ok(Self {
            initializer,
            global_pda,
            mint,
            system_program,
            token_program,
        })
    }
}

impl<'a> Initialize<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0;

    pub fn process(&mut self, _program_id: &Pubkey) -> ProgramResult {
        //// Create GlobalPda account
        let dummy_space: usize = 1; //@note : `temp_globalpda_superflous_space` -> even though no data is stored in global_pda lets just keep it as extra safety against garbage collection
        let global_pda_rent = Rent::get()?.minimum_balance(dummy_space);

        CreateAccount {
            from: self.accounts.initializer,
            to: self.accounts.global_pda,
            lamports: global_pda_rent,
            space: dummy_space as u64,
            owner: _program_id,
            // owner: &crate::ID, // dev :  or you can take programId via process
        }
        .invoke()?;

        //// Create Token Mint
        let mint_space = Mint::LEN;
        let mint_rent = Rent::get()?.minimum_balance(mint_space);

        // account creation
        CreateAccount {
            from: self.accounts.initializer,
            to: self.accounts.mint,
            lamports: mint_rent,
            space: mint_space as u64,
            owner: self.accounts.token_program.key(),
            // owner: &pinocchio_token::ID, // dev: what if initializer wants to create either Legacy or Token2022 .... how will pinocchio_token::ID handle that ???
        }
        .invoke()?;

        // mint initialization
        InitializeMint2 {
            mint: self.accounts.mint,
            decimals: 6,
            mint_authority: self.accounts.global_pda.key(),
            freeze_authority: None,
            // freeze_authority: Some(self.accounts.global_pda.key()), // dev : Lets not bother using freezing logic
        }
        .invoke()
    }
}
