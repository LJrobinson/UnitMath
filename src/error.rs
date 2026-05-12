use std::{error::Error, fmt};

/// Errors returned by UnitMath parsing helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitMathError {
    /// The input was empty after trimming whitespace.
    EmptyInput,
    /// The input did not include a numeric value.
    MissingNumber,
    /// The numeric value could not be parsed.
    InvalidNumber,
    /// The input did not include a unit.
    MissingUnit,
    /// The unit string is not supported.
    UnknownUnit,
}

impl fmt::Display for UnitMathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyInput => "input is empty",
            Self::MissingNumber => "missing numeric value",
            Self::InvalidNumber => "invalid numeric value",
            Self::MissingUnit => "missing unit",
            Self::UnknownUnit => "unknown unit",
        };

        formatter.write_str(message)
    }
}

impl Error for UnitMathError {}
