#![no_std]
use pinocchio::{
    account_info::AccountInfo, entrypoint, nostd_panic_handler, program_error::ProgramError,
    pubkey::Pubkey, ProgramResult,
};

use crate::instructions::Initialize;
pub mod instructions;

nostd_panic_handler!();
entrypoint!(process_instruction);

// 22222222222222222222222222222222222222222222
pub const ID: Pubkey = [
    0x0f, 0x1e, 0x6b, 0x14, 0x21, 0xc0, 0x4a, 0x07, 0x04, 0x31, 0x26, 0x5c, 0x19, 0xc5, 0xbb, 0xee,
    0x19, 0x92, 0xba, 0xe8, 0xaf, 0xd1, 0xcd, 0x07, 0x8e, 0xf8, 0xaf, 0x70, 0x47, 0xdc, 0x11, 0xf7,
];

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    match _instruction_data.split_first() {
        Some((Initialize::DISCRIMINATOR, _ )) => {
            Initialize::try_from(_accounts)?.process(_program_id)?
        }
        _ => Err(ProgramError::InvalidInstructionData)?,
    }
    Ok(())
}
