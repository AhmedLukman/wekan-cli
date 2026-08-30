mod confirmation;
mod payload;

pub use self::confirmation::{
    ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
    SystemConfirmationProvider, confirm_or_skip, escape_terminal_text,
};

pub(crate) use self::payload::{PayloadBody, PayloadSource};

#[cfg(test)]
pub(crate) use self::confirmation::tests::FakeConfirmationProvider;
