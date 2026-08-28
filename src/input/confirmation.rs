use std::io::{self, BufRead, IsTerminal, Write};

use crate::{command_result::DestructiveOperation, error::AppError};

#[derive(Clone, Copy, Debug, Default, clap::Args)]
pub struct ConfirmationArgs {
    /// Skip the destructive-operation confirmation prompt.
    #[arg(long)]
    yes: bool,
}

impl ConfirmationArgs {
    pub const fn assume_yes() -> Self {
        Self { yes: true }
    }

    pub const fn yes(&self) -> bool {
        self.yes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmationDecision {
    Proceed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmationRequest {
    operation: DestructiveOperation,
    prompt: String,
}

impl ConfirmationRequest {
    pub fn new(operation: DestructiveOperation, prompt: impl Into<String>) -> Self {
        Self {
            operation,
            prompt: prompt.into(),
        }
    }

    pub const fn operation(&self) -> DestructiveOperation {
        self.operation
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}

pub trait ConfirmationProvider: Send + Sync {
    fn confirm(&self, request: &ConfirmationRequest) -> Result<bool, AppError>;
}

pub fn confirm_or_skip(
    args: &ConfirmationArgs,
    provider: &dyn ConfirmationProvider,
    request: &ConfirmationRequest,
) -> Result<ConfirmationDecision, AppError> {
    if args.yes {
        return Ok(ConfirmationDecision::Proceed);
    }

    provider.confirm(request).map(|confirmed| {
        if confirmed {
            ConfirmationDecision::Proceed
        } else {
            ConfirmationDecision::Cancelled
        }
    })
}

pub fn escape_terminal_text(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}

#[derive(Clone, Copy, Debug)]
pub struct SystemConfirmationProvider {
    allow_interactive: bool,
}

impl SystemConfirmationProvider {
    pub const fn interactive() -> Self {
        Self {
            allow_interactive: true,
        }
    }

    pub const fn for_output(allow_interactive: bool) -> Self {
        Self { allow_interactive }
    }
}

impl Default for SystemConfirmationProvider {
    fn default() -> Self {
        Self::interactive()
    }
}

impl ConfirmationProvider for SystemConfirmationProvider {
    fn confirm(&self, request: &ConfirmationRequest) -> Result<bool, AppError> {
        if !self.allow_interactive || !io::stdin().is_terminal() || !io::stderr().is_terminal() {
            return Err(AppError::invalid_input(format!(
                "confirmation is required for {}; rerun with --yes",
                request.operation().as_command()
            )));
        }

        read_confirmation(io::stdin().lock(), io::stderr().lock(), request.prompt())
    }
}

fn read_confirmation(
    mut reader: impl BufRead,
    mut writer: impl Write,
    prompt: &str,
) -> Result<bool, AppError> {
    loop {
        write!(writer, "{prompt} [y/N] ")
            .and_then(|()| writer.flush())
            .map_err(|error| {
                AppError::invalid_input(format!("could not write the confirmation prompt: {error}"))
            })?;

        let mut answer = String::new();
        let bytes_read = reader.read_line(&mut answer).map_err(|error| {
            AppError::invalid_input(format!("could not read the confirmation: {error}"))
        })?;
        if bytes_read == 0 {
            return Ok(false);
        }

        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "" | "n" | "no" => return Ok(false),
            _ => {
                writeln!(writer, "Please answer yes or no.").map_err(|error| {
                    AppError::invalid_input(format!(
                        "could not write the confirmation prompt: {error}"
                    ))
                })?;
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{
        io::{self, BufRead, Cursor, Write},
        sync::{Arc, Mutex},
    };

    use super::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip, read_confirmation,
    };

    #[derive(Clone)]
    pub(crate) struct FakeConfirmationProvider {
        response: Arc<Mutex<Result<bool, crate::error::AppError>>>,
        requests: Arc<Mutex<Vec<ConfirmationRequest>>>,
    }

    impl FakeConfirmationProvider {
        pub(crate) fn accepting() -> Self {
            Self::with_response(Ok(true))
        }

        pub(crate) fn declining() -> Self {
            Self::with_response(Ok(false))
        }

        pub(crate) fn failing(error: crate::error::AppError) -> Self {
            Self::with_response(Err(error))
        }

        fn with_response(response: Result<bool, crate::error::AppError>) -> Self {
            Self {
                response: Arc::new(Mutex::new(response)),
                requests: Arc::default(),
            }
        }

        pub(crate) fn requests(&self) -> Vec<ConfirmationRequest> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl ConfirmationProvider for FakeConfirmationProvider {
        fn confirm(&self, request: &ConfirmationRequest) -> Result<bool, crate::error::AppError> {
            self.requests.lock().unwrap().push(request.clone());
            self.response.lock().unwrap().clone()
        }
    }

    #[test]
    fn disabled_interactivity_requires_yes() {
        let provider = super::SystemConfirmationProvider::for_output(false);
        let request = ConfirmationRequest::new(
            crate::command_result::DestructiveOperation::AuthLogout,
            "Continue?",
        );

        let error = provider.confirm(&request).unwrap_err();

        assert_eq!(error.code(), crate::error::ErrorCode::InvalidInput);
        assert!(error.message().contains("--yes"));
        assert!(error.message().contains("auth logout"));
    }

    #[test]
    fn shared_gate_maps_yes_provider_answers_and_errors() {
        let request = ConfirmationRequest::new(
            crate::command_result::DestructiveOperation::ProfileRemove,
            "Continue?",
        );
        let bypassed = FakeConfirmationProvider::failing(crate::error::AppError::invalid_input(
            "must not be called",
        ));
        assert_eq!(
            confirm_or_skip(&ConfirmationArgs::assume_yes(), &bypassed, &request).unwrap(),
            ConfirmationDecision::Proceed
        );
        assert!(bypassed.requests().is_empty());

        let accepting = FakeConfirmationProvider::accepting();
        assert_eq!(
            confirm_or_skip(&ConfirmationArgs::default(), &accepting, &request).unwrap(),
            ConfirmationDecision::Proceed
        );
        let declining = FakeConfirmationProvider::declining();
        assert_eq!(
            confirm_or_skip(&ConfirmationArgs::default(), &declining, &request).unwrap(),
            ConfirmationDecision::Cancelled
        );
        let failing = FakeConfirmationProvider::failing(crate::error::AppError::invalid_input(
            "test failure",
        ));
        assert_eq!(
            confirm_or_skip(&ConfirmationArgs::default(), &failing, &request)
                .unwrap_err()
                .message(),
            "test failure"
        );
    }

    #[test]
    fn accepts_case_insensitive_yes_forms() {
        for answer in ["y\n", "YES\n", " Yes \r\n"] {
            assert!(read_confirmation(Cursor::new(answer), Vec::new(), "Continue?").unwrap());
        }
    }

    #[test]
    fn no_empty_and_eof_cancel() {
        for answer in ["n\n", "NO\n", "\n", ""] {
            assert!(!read_confirmation(Cursor::new(answer), Vec::new(), "Continue?").unwrap());
        }
    }

    #[test]
    fn invalid_answers_are_retried() {
        let mut output = Vec::new();
        assert!(read_confirmation(Cursor::new("maybe\nyes\n"), &mut output, "Continue?").unwrap());
        let output = String::from_utf8(output).unwrap();
        assert_eq!(output.matches("Continue? [y/N]").count(), 2);
        assert!(output.contains("Please answer yes or no."));
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("test write failure"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn prompt_write_failures_are_invalid_input() {
        let error =
            read_confirmation(Cursor::new("yes\n"), FailingWriter, "Continue?").unwrap_err();
        assert!(error.message().contains("test write failure"));
    }

    struct FailingReader;

    impl io::Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("test read failure"))
        }
    }

    impl BufRead for FailingReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Err(io::Error::other("test read failure"))
        }

        fn consume(&mut self, _amount: usize) {}
    }

    #[test]
    fn prompt_read_failures_are_invalid_input() {
        let error = read_confirmation(FailingReader, Vec::new(), "Continue?").unwrap_err();
        assert!(error.message().contains("test read failure"));
    }

    #[test]
    fn terminal_text_escapes_control_characters() {
        assert_eq!(
            super::escape_terminal_text("board\u{1b}]52;c;clipboard\u{7}\n"),
            r"board\u{1b}]52;c;clipboard\u{7}\n"
        );
    }
}
