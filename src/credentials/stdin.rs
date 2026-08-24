use std::io::BufRead;

use secrecy::SecretString;

use crate::error::AppError;

pub(super) fn read_password_line(mut reader: impl BufRead) -> Result<SecretString, AppError> {
    let mut password = String::new();
    let bytes_read = reader.read_line(&mut password).map_err(|error| {
        AppError::invalid_input(format!("could not read the password from stdin: {error}"))
    })?;
    if bytes_read == 0 {
        return Err(AppError::invalid_input(
            "standard input ended before a password was read",
        ));
    }
    if password.ends_with('\n') {
        password.pop();
        if password.ends_with('\r') {
            password.pop();
        }
    }

    super::validate_password(password, None)
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::read_password_line;

    #[test]
    fn removes_only_the_line_ending() {
        let password = read_password_line(&b"  keep spaces  \r\n"[..]).unwrap();
        assert_eq!(password.expose_secret(), "  keep spaces  ");
    }

    #[test]
    fn rejects_empty_input() {
        assert!(read_password_line(&b""[..]).is_err());
        assert!(read_password_line(&b"\n"[..]).is_err());
    }
}
