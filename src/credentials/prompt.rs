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
