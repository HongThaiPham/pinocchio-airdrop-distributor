use pinocchio::program_error::ProgramError;

#[derive(Clone, PartialEq)]
pub enum AirdropProgramError {
    InvalidProof,
    Unauthorized,
}

impl From<AirdropProgramError> for ProgramError {
    fn from(e: AirdropProgramError) -> Self {
        Self::Custom(e as u32)
    }
}
