use std::io::BufRead;

use secrecy::SecretString;

use crate::error::AppError;

pub(super) fn read_password_line(mut reader: impl BufRead) -> Result<SecretString, AppError> {
    let password = read_line(&mut reader, "password")?;
    super::validate_password(password, None)
}

pub(super) fn read_code_line(mut reader: impl BufRead) -> Result<SecretString, AppError> {
    let code = read_line(&mut reader, "two-factor code")?;
    super::validate_code(code)
}

fn read_line(reader: &mut impl BufRead, field: &str) -> Result<String, AppError> {
    let mut value = String::new();
    let bytes_read = reader.read_line(&mut value).map_err(|error| {
        AppError::invalid_input(format!("could not read the {field} from stdin: {error}"))
    })?;
    if bytes_read == 0 {
        return Err(AppError::invalid_input(format!(
            "standard input ended before a {field} was read"
        )));
    }
    if value.ends_with('\n') {
        value.pop();
        if value.ends_with('\r') {
            value.pop();
        }
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::{read_code_line, read_password_line};

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

    #[test]
    fn reads_password_and_code_lines_in_order() {
        let mut input = &b"  password  \r\n 123456 \n"[..];
        let password = read_password_line(&mut input).unwrap();
        let code = read_code_line(&mut input).unwrap();

        assert_eq!(password.expose_secret(), "  password  ");
        assert_eq!(code.expose_secret(), " 123456 ");
    }

    #[test]
    fn rejects_empty_or_missing_codes() {
        assert!(read_code_line(&b""[..]).is_err());
        assert!(read_code_line(&b"\r\n"[..]).is_err());
    }
}
