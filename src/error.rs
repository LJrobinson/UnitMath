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
