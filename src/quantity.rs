use crate::{
    convert_potency, convert_volume, convert_weight, PotencyUnit, UnitMathError, VolumeUnit,
    WeightUnit,
};

/// A parsed numeric quantity paired with a unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParsedQuantity<U> {
    /// The parsed numeric value.
    pub value: f64,
    /// The parsed unit.
    pub unit: U,
}

/// Parses a weight quantity from a string.
///
/// Supported units are `mg`, `g`, `gram`, `grams`, `kg`, `oz`, `ounce`,
/// `ounces`, `lb`, `lbs`, `pound`, and `pounds`.
pub fn parse_weight(input: &str) -> Result<ParsedQuantity<WeightUnit>, UnitMathError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(UnitMathError::EmptyInput);
    }

    let Some(unit_start) = trimmed
        .char_indices()
        .find(|(_, character)| character.is_ascii_alphabetic())
        .map(|(index, _)| index)
    else {
        return if trimmed.parse::<f64>().is_ok() {
            Err(UnitMathError::MissingUnit)
        } else {
            Err(UnitMathError::MissingNumber)
        };
    };

    if unit_start == 0 {
        return Err(UnitMathError::MissingNumber);
    }

    let value = trimmed[..unit_start]
        .trim()
        .parse::<f64>()
        .map_err(|_| UnitMathError::InvalidNumber)?;
    let unit_text = trimmed[unit_start..].trim().to_ascii_lowercase();

    let unit = match unit_text.as_str() {
        "mg" => WeightUnit::Milligram,
        "g" | "gram" | "grams" => WeightUnit::Gram,
        "kg" => WeightUnit::Kilogram,
        "oz" | "ounce" | "ounces" => WeightUnit::Ounce,
        "lb" | "lbs" | "pound" | "pounds" => WeightUnit::Pound,
        _ => return Err(UnitMathError::UnknownUnit),
    };

    Ok(ParsedQuantity { value, unit })
}

/// Parses a volume quantity from a string.
///
/// Supported units are `ml`, `milliliter`, `milliliters`, `l`, `liter`,
/// `liters`, `fl oz`, `floz`, `fluid ounce`, `fluid ounces`, `cup`, `cups`,
/// `pint`, `pints`, `quart`, `quarts`, `gallon`, and `gallons`.
pub fn parse_volume(input: &str) -> Result<ParsedQuantity<VolumeUnit>, UnitMathError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(UnitMathError::EmptyInput);
    }

    let Some(unit_start) = trimmed
        .char_indices()
        .find(|(_, character)| character.is_ascii_alphabetic())
        .map(|(index, _)| index)
    else {
        return if trimmed.parse::<f64>().is_ok() {
            Err(UnitMathError::MissingUnit)
        } else {
            Err(UnitMathError::MissingNumber)
        };
    };

    if unit_start == 0 {
        return Err(UnitMathError::MissingNumber);
    }

    let value = trimmed[..unit_start]
        .trim()
        .parse::<f64>()
        .map_err(|_| UnitMathError::InvalidNumber)?;
    let unit_text = trimmed[unit_start..].trim().to_ascii_lowercase();

    let unit = match unit_text.as_str() {
        "ml" | "milliliter" | "milliliters" => VolumeUnit::Milliliter,
        "l" | "liter" | "liters" => VolumeUnit::Liter,
        "fl oz" | "floz" | "fluid ounce" | "fluid ounces" => VolumeUnit::FluidOunce,
        "cup" | "cups" => VolumeUnit::Cup,
        "pint" | "pints" => VolumeUnit::Pint,
        "quart" | "quarts" => VolumeUnit::Quart,
        "gallon" | "gallons" => VolumeUnit::Gallon,
        _ => return Err(UnitMathError::UnknownUnit),
    };

    Ok(ParsedQuantity { value, unit })
}

/// Parses a potency quantity from a string.
///
/// Supported units are `%`, `percent`, `percentage`, `mg/g`, `mgg`,
/// `mg per g`, `mg/g dry weight`, and `milligrams per gram`.
pub fn parse_potency(input: &str) -> Result<ParsedQuantity<PotencyUnit>, UnitMathError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(UnitMathError::EmptyInput);
    }

    let Some(unit_start) = trimmed
        .char_indices()
        .find(|(_, character)| character.is_ascii_alphabetic() || *character == '%')
        .map(|(index, _)| index)
    else {
        return if trimmed.parse::<f64>().is_ok() {
            Err(UnitMathError::MissingUnit)
        } else {
            Err(UnitMathError::MissingNumber)
        };
    };

    if unit_start == 0 {
        return Err(UnitMathError::MissingNumber);
    }

    let value = trimmed[..unit_start]
        .trim()
        .parse::<f64>()
        .map_err(|_| UnitMathError::InvalidNumber)?;
    let unit_text = trimmed[unit_start..].trim().to_ascii_lowercase();

    let unit = match unit_text.as_str() {
        "%" | "percent" | "percentage" => PotencyUnit::Percent,
        "mg/g" | "mgg" | "mg per g" | "mg/g dry weight" | "milligrams per gram" => {
            PotencyUnit::MilligramsPerGram
        }
        _ => return Err(UnitMathError::UnknownUnit),
    };

    Ok(ParsedQuantity { value, unit })
}

/// Parses a weight quantity string and converts it to the target weight unit.
pub fn convert_parsed_weight(input: &str, to: WeightUnit) -> Result<f64, UnitMathError> {
    let parsed = parse_weight(input)?;

    Ok(convert_weight(parsed.value, parsed.unit, to))
}

/// Parses a volume quantity string and converts it to the target volume unit.
pub fn convert_parsed_volume(input: &str, to: VolumeUnit) -> Result<f64, UnitMathError> {
    let parsed = parse_volume(input)?;

    Ok(convert_volume(parsed.value, parsed.unit, to))
}

/// Parses a potency quantity string and converts it to the target potency unit.
pub fn convert_parsed_potency(input: &str, to: PotencyUnit) -> Result<f64, UnitMathError> {
    let parsed = parse_potency(input)?;

    Ok(convert_potency(parsed.value, parsed.unit, to))
}

#[cfg(test)]
mod tests {
    use super::{
        convert_parsed_potency, convert_parsed_volume, convert_parsed_weight, parse_potency,
        parse_volume, parse_weight, ParsedQuantity,
    };
    use crate::{PotencyUnit, UnitMathError, VolumeUnit, WeightUnit};

    fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
        let difference = (actual - expected).abs();
        assert!(
            difference <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}; difference was {difference}"
        );
    }

    fn assert_parsed_weight(input: &str, expected_value: f64, expected_unit: WeightUnit) {
        let ParsedQuantity { value, unit } = parse_weight(input).unwrap();

        assert_approx_eq(value, expected_value, 1e-12);
        assert_eq!(unit, expected_unit);
    }

    fn assert_parsed_volume(input: &str, expected_value: f64, expected_unit: VolumeUnit) {
        let ParsedQuantity { value, unit } = parse_volume(input).unwrap();

        assert_approx_eq(value, expected_value, 1e-12);
        assert_eq!(unit, expected_unit);
    }

    fn assert_parsed_potency(input: &str, expected_value: f64, expected_unit: PotencyUnit) {
        let ParsedQuantity { value, unit } = parse_potency(input).unwrap();

        assert_approx_eq(value, expected_value, 1e-12);
        assert_eq!(unit, expected_unit);
    }

    #[test]
    fn parses_spaced_milligrams() {
        assert_parsed_weight("1000 mg", 1000.0, WeightUnit::Milligram);
    }

    #[test]
    fn parses_compact_milligrams() {
        assert_parsed_weight("1000mg", 1000.0, WeightUnit::Milligram);
    }

    #[test]
    fn parses_spaced_grams() {
        assert_parsed_weight("3.5 g", 3.5, WeightUnit::Gram);
    }

    #[test]
    fn parses_compact_grams() {
        assert_parsed_weight("3.5g", 3.5, WeightUnit::Gram);
    }

    #[test]
    fn parses_ounces() {
        assert_parsed_weight("1 oz", 1.0, WeightUnit::Ounce);
    }

    #[test]
    fn parses_pounds() {
        assert_parsed_weight("2 pounds", 2.0, WeightUnit::Pound);
    }

    #[test]
    fn parses_uppercase_weight_units() {
        assert_parsed_weight("3.5G", 3.5, WeightUnit::Gram);
    }

    #[test]
    fn parses_weight_with_extra_whitespace() {
        assert_parsed_weight("  1000 mg  ", 1000.0, WeightUnit::Milligram);
    }

    #[test]
    fn parses_weight_decimal_without_leading_zero() {
        assert_parsed_weight(".5 g", 0.5, WeightUnit::Gram);
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(parse_weight("").unwrap_err(), UnitMathError::EmptyInput);
    }

    #[test]
    fn rejects_missing_number() {
        assert_eq!(
            parse_weight("abc").unwrap_err(),
            UnitMathError::MissingNumber
        );
    }

    #[test]
    fn rejects_missing_unit() {
        assert_eq!(parse_weight("100").unwrap_err(), UnitMathError::MissingUnit);
    }

    #[test]
    fn rejects_unknown_unit() {
        assert_eq!(
            parse_weight("10 bananas").unwrap_err(),
            UnitMathError::UnknownUnit
        );
    }

    #[test]
    fn parses_spaced_milliliters() {
        assert_parsed_volume("1000 ml", 1000.0, VolumeUnit::Milliliter);
    }

    #[test]
    fn parses_compact_milliliters() {
        assert_parsed_volume("1000ml", 1000.0, VolumeUnit::Milliliter);
    }

    #[test]
    fn parses_liters_abbreviation() {
        assert_parsed_volume("1 l", 1.0, VolumeUnit::Liter);
    }

    #[test]
    fn parses_liters_name() {
        assert_parsed_volume("1 liter", 1.0, VolumeUnit::Liter);
    }

    #[test]
    fn parses_spaced_fluid_ounces() {
        assert_parsed_volume("8 fl oz", 8.0, VolumeUnit::FluidOunce);
    }

    #[test]
    fn parses_compact_fluid_ounces() {
        assert_parsed_volume("8floz", 8.0, VolumeUnit::FluidOunce);
    }

    #[test]
    fn parses_cups() {
        assert_parsed_volume("1 cup", 1.0, VolumeUnit::Cup);
    }

    #[test]
    fn parses_gallons() {
        assert_parsed_volume("2 gallons", 2.0, VolumeUnit::Gallon);
    }

    #[test]
    fn parses_uppercase_volume_units() {
        assert_parsed_volume("8 FL OZ", 8.0, VolumeUnit::FluidOunce);
    }

    #[test]
    fn rejects_empty_volume_input() {
        assert_eq!(parse_volume("").unwrap_err(), UnitMathError::EmptyInput);
    }

    #[test]
    fn rejects_volume_missing_number() {
        assert_eq!(
            parse_volume("abc").unwrap_err(),
            UnitMathError::MissingNumber
        );
    }

    #[test]
    fn rejects_volume_missing_unit() {
        assert_eq!(parse_volume("100").unwrap_err(), UnitMathError::MissingUnit);
    }

    #[test]
    fn rejects_unknown_volume_unit() {
        assert_eq!(
            parse_volume("10 buckets").unwrap_err(),
            UnitMathError::UnknownUnit
        );
    }

    #[test]
    fn parses_compact_percent_symbol() {
        assert_parsed_potency("22.4%", 22.4, PotencyUnit::Percent);
    }

    #[test]
    fn parses_spaced_percent_symbol() {
        assert_parsed_potency("22.4 %", 22.4, PotencyUnit::Percent);
    }

    #[test]
    fn parses_percent_name() {
        assert_parsed_potency("22.4 percent", 22.4, PotencyUnit::Percent);
    }

    #[test]
    fn parses_spaced_milligrams_per_gram() {
        assert_parsed_potency("224 mg/g", 224.0, PotencyUnit::MilligramsPerGram);
    }

    #[test]
    fn parses_compact_milligrams_per_gram() {
        assert_parsed_potency("224mg/g", 224.0, PotencyUnit::MilligramsPerGram);
    }

    #[test]
    fn parses_milligrams_per_g() {
        assert_parsed_potency("224 mg per g", 224.0, PotencyUnit::MilligramsPerGram);
    }

    #[test]
    fn parses_milligrams_per_gram() {
        assert_parsed_potency(
            "224 milligrams per gram",
            224.0,
            PotencyUnit::MilligramsPerGram,
        );
    }

    #[test]
    fn parses_uppercase_potency_units() {
        assert_parsed_potency("224 MG/G", 224.0, PotencyUnit::MilligramsPerGram);
    }

    #[test]
    fn rejects_empty_potency_input() {
        assert_eq!(parse_potency("").unwrap_err(), UnitMathError::EmptyInput);
    }

    #[test]
    fn rejects_potency_missing_number() {
        assert_eq!(
            parse_potency("abc").unwrap_err(),
            UnitMathError::MissingNumber
        );
    }

    #[test]
    fn rejects_potency_missing_unit() {
        assert_eq!(
            parse_potency("100").unwrap_err(),
            UnitMathError::MissingUnit
        );
    }

    #[test]
    fn rejects_unknown_potency_unit() {
        assert_eq!(
            parse_potency("10 bananas").unwrap_err(),
            UnitMathError::UnknownUnit
        );
    }

    #[test]
    fn converts_parsed_milligrams_to_grams() {
        let actual = convert_parsed_weight("1000mg", WeightUnit::Gram).unwrap();

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_parsed_grams_to_ounces() {
        let actual = convert_parsed_weight("3.5g", WeightUnit::Ounce).unwrap();

        assert_approx_eq(actual, 0.123_459, 1e-6);
    }

    #[test]
    fn converts_parsed_pounds_to_ounces() {
        let actual = convert_parsed_weight("1 lb", WeightUnit::Ounce).unwrap();

        assert_approx_eq(actual, 16.0, 1e-12);
    }

    #[test]
    fn converts_parsed_milliliters_to_liters() {
        let actual = convert_parsed_volume("1000ml", VolumeUnit::Liter).unwrap();

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_parsed_gallons_to_milliliters() {
        let actual = convert_parsed_volume("1 gallon", VolumeUnit::Milliliter).unwrap();

        assert_approx_eq(actual, 3785.411_784, 1e-12);
    }

    #[test]
    fn converts_parsed_fluid_ounces_to_cups() {
        let actual = convert_parsed_volume("8 fl oz", VolumeUnit::Cup).unwrap();

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_parsed_percent_to_milligrams_per_gram() {
        let actual = convert_parsed_potency("22.4%", PotencyUnit::MilligramsPerGram).unwrap();

        assert_approx_eq(actual, 224.0, 1e-12);
    }

    #[test]
    fn converts_parsed_milligrams_per_gram_to_percent() {
        let actual = convert_parsed_potency("224mg/g", PotencyUnit::Percent).unwrap();

        assert_approx_eq(actual, 22.4, 1e-12);
    }

    #[test]
    fn returns_weight_parse_errors() {
        assert!(convert_parsed_weight("10 bananas", WeightUnit::Gram).is_err());
    }

    #[test]
    fn returns_volume_parse_errors() {
        assert!(convert_parsed_volume("10 buckets", VolumeUnit::Liter).is_err());
    }

    #[test]
    fn returns_potency_parse_errors() {
        assert!(convert_parsed_potency("10 bananas", PotencyUnit::Percent).is_err());
    }
}
