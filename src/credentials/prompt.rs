use secrecy::SecretString;

use crate::error::AppError;

pub(super) fn read_confirmed_password() -> Result<SecretString, AppError> {
    let password = rpassword::prompt_password("Password: ").map_err(|error| {
        AppError::invalid_input(format!("could not read the password: {error}"))
    })?;
    let confirmation = rpassword::prompt_password("Confirm password: ").map_err(|error| {
        AppError::invalid_input(format!("could not read the password confirmation: {error}"))
    })?;

    super::validate_password(password, Some(confirmation))
}

pub(super) fn read_password() -> Result<SecretString, AppError> {
    let password = rpassword::prompt_password("Password: ").map_err(|error| {
        AppError::invalid_input(format!("could not read the password: {error}"))
    })?;

    super::validate_password(password, None)
}

pub(super) fn read_code() -> Result<SecretString, AppError> {
    let code = rpassword::prompt_password("Two-factor code: ").map_err(|error| {
        AppError::invalid_input(format!("could not read the two-factor code: {error}"))
    })?;

    super::validate_code(code)
}
