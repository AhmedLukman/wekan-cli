mod confirmation;

pub use self::confirmation::{
    ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
    SystemConfirmationProvider, confirm_or_skip, escape_terminal_text,
};

#[cfg(test)]
pub(crate) use self::confirmation::tests::FakeConfirmationProvider;
