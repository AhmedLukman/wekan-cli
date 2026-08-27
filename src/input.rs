mod confirmation;

pub use self::confirmation::{
    ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
    SystemConfirmationProvider, confirm_or_skip,
};

#[cfg(test)]
pub(crate) use self::confirmation::tests::FakeConfirmationProvider;
