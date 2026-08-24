#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum StableExitCode {
    Internal = 1,
    InvalidInput = 2,
    Configuration = 3,
    Transport = 4,
    Server = 5,
    Credential = 6,
}

impl From<StableExitCode> for std::process::ExitCode {
    fn from(value: StableExitCode) -> Self {
        Self::from(value as u8)
    }
}
