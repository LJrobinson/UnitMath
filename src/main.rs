use std::{env, process};

use unitmath::{
    convert_parsed_potency, convert_parsed_volume, convert_parsed_weight, PotencyUnit,
    UnitMathError, VolumeUnit, WeightUnit,
};

fn main() {
    match run(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(value) => println!("{value}"),
        Err(message) => {
            eprintln!("{message}");
            eprintln!("{}", usage());
            process::exit(1);
        }
    }
}

fn run(args: &[String]) -> Result<f64, String> {
    if args.len() < 3 {
        return Err("missing arguments".to_string());
    }

    let category = args[0].trim().to_ascii_lowercase();
    let input = &args[1];
    let target = args[2..].join(" ");

    match category.as_str() {
        "weight" => {
            let unit = parse_target_weight_unit(&target)
                .ok_or_else(|| format!("unknown weight target unit: {target}"))?;
            convert_parsed_weight(input, unit).map_err(|error| parse_error_message("weight", error))
        }
        "volume" => {
            let unit = parse_target_volume_unit(&target)
                .ok_or_else(|| format!("unknown volume target unit: {target}"))?;
            convert_parsed_volume(input, unit).map_err(|error| parse_error_message("volume", error))
        }
        "potency" => {
            let unit = parse_target_potency_unit(&target)
                .ok_or_else(|| format!("unknown potency target unit: {target}"))?;
            convert_parsed_potency(input, unit)
                .map_err(|error| parse_error_message("potency", error))
        }
        _ => Err(format!("unknown command: {}", args[0])),
    }
}

fn parse_target_weight_unit(input: &str) -> Option<WeightUnit> {
    match input.trim().to_ascii_lowercase().as_str() {
        "mg" => Some(WeightUnit::Milligram),
        "g" => Some(WeightUnit::Gram),
        "kg" => Some(WeightUnit::Kilogram),
        "oz" => Some(WeightUnit::Ounce),
        "lb" => Some(WeightUnit::Pound),
        _ => None,
    }
}

fn parse_target_volume_unit(input: &str) -> Option<VolumeUnit> {
    match input.trim().to_ascii_lowercase().as_str() {
        "ml" => Some(VolumeUnit::Milliliter),
        "l" => Some(VolumeUnit::Liter),
        "fl oz" | "floz" => Some(VolumeUnit::FluidOunce),
        "cup" => Some(VolumeUnit::Cup),
        "pint" => Some(VolumeUnit::Pint),
        "quart" => Some(VolumeUnit::Quart),
        "gallon" => Some(VolumeUnit::Gallon),
        _ => None,
    }
}

fn parse_target_potency_unit(input: &str) -> Option<PotencyUnit> {
    match input.trim().to_ascii_lowercase().as_str() {
        "%" | "percent" => Some(PotencyUnit::Percent),
        "mg/g" | "mgg" => Some(PotencyUnit::MilligramsPerGram),
        _ => None,
    }
}

fn parse_error_message(category: &str, error: UnitMathError) -> String {
    format!("failed to parse {category} input: {error}")
}

fn usage() -> &'static str {
    r#"Usage:
  unitmath weight "<input>" <target-unit>
  unitmath volume "<input>" <target-unit>
  unitmath potency "<input>" <target-unit>

Target units:
  weight: mg, g, kg, oz, lb
  volume: ml, l, "fl oz", floz, cup, pint, quart, gallon
  potency: %, percent, mg/g, mgg"#
}

#[cfg(test)]
mod tests {
    use super::run;

    fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
        let difference = (actual - expected).abs();
        assert!(
            difference <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}; difference was {difference}"
        );
    }

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn converts_weight_input() {
        let actual = run(&args(&["weight", "1000mg", "g"])).unwrap();

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_volume_input() {
        let actual = run(&args(&["volume", "8 fl oz", "cup"])).unwrap();

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_volume_target_split_by_shell() {
        let actual = run(&args(&["volume", "1 cup", "fl", "oz"])).unwrap();

        assert_approx_eq(actual, 8.0, 1e-12);
    }

    #[test]
    fn converts_potency_input() {
        let actual = run(&args(&["potency", "22.4%", "mg/g"])).unwrap();

        assert_approx_eq(actual, 224.0, 1e-12);
    }

    #[test]
    fn rejects_unknown_target_unit() {
        assert!(run(&args(&["volume", "1000ml", "bucket"])).is_err());
    }

    #[test]
    fn rejects_parse_errors() {
        assert!(run(&args(&["weight", "10 bananas", "g"])).is_err());
    }

    #[test]
    fn rejects_missing_arguments() {
        assert!(run(&args(&["weight", "1000mg"])).is_err());
    }

    #[test]
    fn rejects_no_arguments() {
        assert!(run(&[]).is_err());
    }
}
