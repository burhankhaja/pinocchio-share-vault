#![no_std]
use pinocchio::{entrypoint, nostd_panic_handler, ProgramResult, pubkey::Pubkey, account_info::AccountInfo};
// pub mod instructions;

nostd_panic_handler!();
entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _data: &[u8],
) -> ProgramResult {
    Ok(())
}