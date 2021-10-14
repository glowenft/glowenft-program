use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum GloweError{
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,

    /// Account was not rent exempt
    #[error("Provided account is not rent exempt")]
    NotRentExempt,

    /// Provided account did not match the expected account
    #[error("Provided account did not match the expected account")]
    AccountMismatch,
}

impl From<GloweError> for ProgramError {
    fn from(from: GloweError) -> Self {
        Self::Custom(from as u32)
    }
}
