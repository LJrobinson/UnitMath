use std::{env, fmt::Write, process};

use unitmath::{
    convert_parsed_potency, convert_parsed_volume, convert_parsed_weight, PotencyUnit,
    UnitMathError, VolumeUnit, WeightUnit,
};

fn main() {
    match run(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(output) => println!("{}", output.render()),
        Err(message) => {
            eprintln!("{message}");
            eprintln!("{}", usage());
            process::exit(1);
        }
    }
}

#[derive(Debug, PartialEq)]
struct CliOutput {
    category: String,
    input: String,
    target_unit: String,
    value: f64,
    format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Plain,
    Json,
    Csv,
}

impl CliOutput {
    fn render(&self) -> String {
        match self.format {
            OutputFormat::Plain => self.value.to_string(),
            OutputFormat::Json => format!(
                r#"{{"category":"{}","input":"{}","target_unit":"{}","value":{}}}"#,
                escape_json_string(&self.category),
                escape_json_string(&self.input),
                escape_json_string(&self.target_unit),
                self.value
            ),
            OutputFormat::Csv => format!(
                "category,input,target_unit,value\n{},{},{},{}",
                escape_csv_field(&self.category),
                escape_csv_field(&self.input),
                escape_csv_field(&self.target_unit),
                self.value
            ),
        }
    }
}

fn run(args: &[String]) -> Result<CliOutput, String> {
    if args.len() < 3 {
        return Err("missing arguments".to_string());
    }

    let has_json = args.iter().any(|argument| argument == "--json");
    let has_csv = args.iter().any(|argument| argument == "--csv");

    if has_json && has_csv {
        return Err("--json and --csv cannot be used together".to_string());
    }

    let format = match args.last().map(String::as_str) {
        Some("--json") => OutputFormat::Json,
        Some("--csv") => OutputFormat::Csv,
        _ => OutputFormat::Plain,
    };
    let command_args = match format {
        OutputFormat::Json | OutputFormat::Csv => &args[..args.len() - 1],
        OutputFormat::Plain => args,
    };

    if command_args.iter().any(|argument| argument == "--json") {
        return Err("--json must appear at the end of the command".to_string());
    }

    if command_args.iter().any(|argument| argument == "--csv") {
        return Err("--csv must appear at the end of the command".to_string());
    }

    if command_args.len() < 3 {
        return Err("missing arguments".to_string());
    }

    let category = command_args[0].trim().to_ascii_lowercase();
    let input = &command_args[1];
    let target = command_args[2..].join(" ");

    let (output_category, value) = match category.as_str() {
        "weight" => {
            let unit = parse_target_weight_unit(&target)
                .ok_or_else(|| format!("unknown weight target unit: {target}"))?;
            let value = convert_parsed_weight(input, unit)
                .map_err(|error| parse_error_message("weight", error))?;

            Ok(("weight".to_string(), value))
        }
        "volume" => {
            let unit = parse_target_volume_unit(&target)
                .ok_or_else(|| format!("unknown volume target unit: {target}"))?;
            let value = convert_parsed_volume(input, unit)
                .map_err(|error| parse_error_message("volume", error))?;

            Ok(("volume".to_string(), value))
        }
        "potency" => {
            let unit = parse_target_potency_unit(&target)
                .ok_or_else(|| format!("unknown potency target unit: {target}"))?;
            let value = convert_parsed_potency(input, unit)
                .map_err(|error| parse_error_message("potency", error))?;

            Ok(("potency".to_string(), value))
        }
        "convert" => infer_conversion(input, &target),
        _ => Err(format!("unknown command: {}", args[0])),
    }?;

    Ok(CliOutput {
        category: output_category,
        input: input.to_string(),
        target_unit: target.trim().to_string(),
        value,
        format,
    })
}

fn infer_conversion(input: &str, target: &str) -> Result<(String, f64), String> {
    let mut matches = Vec::new();

    if let Some(unit) = parse_target_weight_unit(target) {
        if let Ok(value) = convert_parsed_weight(input, unit) {
            matches.push(("weight".to_string(), value));
        }
    }

    if let Some(unit) = parse_target_volume_unit(target) {
        if let Ok(value) = convert_parsed_volume(input, unit) {
            matches.push(("volume".to_string(), value));
        }
    }

    if let Some(unit) = parse_target_potency_unit(target) {
        if let Ok(value) = convert_parsed_potency(input, unit) {
            matches.push(("potency".to_string(), value));
        }
    }

    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(format!(
            "could not infer conversion category for input '{input}' and target unit '{target}'"
        )),
        _ => Err(format!(
            "ambiguous conversion for input '{input}' and target unit '{target}'"
        )),
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

fn escape_json_string(input: &str) -> String {
    let mut escaped = String::new();

    for character in input.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            character if character <= '\u{1f}' => {
                write!(&mut escaped, "\\u{:04x}", character as u32).unwrap();
            }
            character => escaped.push(character),
        }
    }

    escaped
}

fn escape_csv_field(input: &str) -> String {
    if input
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", input.replace('"', "\"\""))
    } else {
        input.to_string()
    }
}

fn usage() -> &'static str {
    r#"Usage:
  unitmath weight "<input>" <target-unit>
  unitmath volume "<input>" <target-unit>
  unitmath potency "<input>" <target-unit>
  unitmath convert "<input>" <target-unit>
  unitmath weight "<input>" <target-unit> --json
  unitmath volume "<input>" <target-unit> --json
  unitmath potency "<input>" <target-unit> --json
  unitmath convert "<input>" <target-unit> --json
  unitmath weight "<input>" <target-unit> --csv
  unitmath volume "<input>" <target-unit> --csv
  unitmath potency "<input>" <target-unit> --csv
  unitmath convert "<input>" <target-unit> --csv

Target units:
  weight: mg, g, kg, oz, lb
  volume: ml, l, "fl oz", floz, cup, pint, quart, gallon
  potency: %, percent, mg/g, mgg"#
}

#[cfg(test)]
mod tests {
    use super::{escape_csv_field, escape_json_string, run, CliOutput, OutputFormat};

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
        let actual = run(&args(&["weight", "1000mg", "g"])).unwrap().value;

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_volume_input() {
        let actual = run(&args(&["volume", "8 fl oz", "cup"])).unwrap().value;

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_volume_target_split_by_shell() {
        let actual = run(&args(&["volume", "1 cup", "fl", "oz"])).unwrap().value;

        assert_approx_eq(actual, 8.0, 1e-12);
    }

    #[test]
    fn converts_potency_input() {
        let actual = run(&args(&["potency", "22.4%", "mg/g"])).unwrap().value;

        assert_approx_eq(actual, 224.0, 1e-12);
    }

    #[test]
    fn renders_plain_output_by_default() {
        let output = run(&args(&["weight", "1000mg", "g"])).unwrap();

        assert_eq!(output.render(), "1");
    }

    #[test]
    fn renders_weight_json_output() {
        let output = run(&args(&["weight", "3.5g", "oz", "--json"])).unwrap();

        assert_eq!(
            output.render(),
            r#"{"category":"weight","input":"3.5g","target_unit":"oz","value":0.12345886682353144}"#
        );
    }

    #[test]
    fn renders_volume_json_output() {
        let output = run(&args(&["volume", "8 fl oz", "cup", "--json"])).unwrap();

        assert_eq!(
            output.render(),
            r#"{"category":"volume","input":"8 fl oz","target_unit":"cup","value":1}"#
        );
    }

    #[test]
    fn renders_potency_json_output() {
        let output = run(&args(&["potency", "22.4%", "mg/g", "--json"])).unwrap();

        assert_eq!(
            output.render(),
            r#"{"category":"potency","input":"22.4%","target_unit":"mg/g","value":224}"#
        );
    }

    #[test]
    fn escapes_json_string_fields() {
        let output = CliOutput {
            category: "weight".to_string(),
            input: r#"3.5"g\"#.to_string(),
            target_unit: "oz".to_string(),
            value: 1.0,
            format: OutputFormat::Json,
        };

        assert_eq!(
            output.render(),
            r#"{"category":"weight","input":"3.5\"g\\","target_unit":"oz","value":1}"#
        );
        assert_eq!(escape_json_string("line\nbreak"), "line\\nbreak");
    }

    #[test]
    fn rejects_invalid_json_flag_placement() {
        assert!(run(&args(&["weight", "1000mg", "--json", "g"])).is_err());
    }

    #[test]
    fn renders_weight_csv_output() {
        let output = run(&args(&["weight", "1000mg", "g", "--csv"])).unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value\nweight,1000mg,g,1"
        );
    }

    #[test]
    fn renders_volume_csv_output() {
        let output = run(&args(&["volume", "8 fl oz", "cup", "--csv"])).unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value\nvolume,8 fl oz,cup,1"
        );
    }

    #[test]
    fn renders_potency_csv_output() {
        let output = run(&args(&["potency", "22.4%", "mg/g", "--csv"])).unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value\npotency,22.4%,mg/g,224"
        );
    }

    #[test]
    fn renders_universal_csv_output() {
        let output = run(&args(&["convert", "1000mg", "g", "--csv"])).unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value\nweight,1000mg,g,1"
        );
    }

    #[test]
    fn escapes_csv_fields_with_commas() {
        let output = CliOutput {
            category: "weight".to_string(),
            input: "3,5g".to_string(),
            target_unit: "oz".to_string(),
            value: 1.0,
            format: OutputFormat::Csv,
        };

        assert_eq!(
            output.render(),
            "category,input,target_unit,value\nweight,\"3,5g\",oz,1"
        );
    }

    #[test]
    fn escapes_csv_fields_with_quotes() {
        let output = CliOutput {
            category: "weight".to_string(),
            input: r#"3.5"g"#.to_string(),
            target_unit: "oz".to_string(),
            value: 1.0,
            format: OutputFormat::Csv,
        };

        assert_eq!(
            output.render(),
            "category,input,target_unit,value\nweight,\"3.5\"\"g\",oz,1"
        );
        assert_eq!(escape_csv_field("line\nbreak"), "\"line\nbreak\"");
    }

    #[test]
    fn rejects_json_and_csv_together() {
        assert!(run(&args(&["weight", "1000mg", "g", "--json", "--csv"])).is_err());
    }

    #[test]
    fn rejects_invalid_csv_flag_placement() {
        assert!(run(&args(&["weight", "1000mg", "--csv", "g"])).is_err());
    }

    #[test]
    fn converts_universal_weight_input() {
        let actual = run(&args(&["convert", "1000mg", "g"])).unwrap().value;

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_universal_volume_input() {
        let actual = run(&args(&["convert", "1 gallon", "ml"])).unwrap().value;

        assert_approx_eq(actual, 3785.411_784, 1e-12);
    }

    #[test]
    fn converts_universal_potency_input() {
        let actual = run(&args(&["convert", "22.4%", "mg/g"])).unwrap().value;

        assert_approx_eq(actual, 224.0, 1e-12);
    }

    #[test]
    fn renders_universal_weight_json_category() {
        let output = run(&args(&["convert", "3.5g", "oz", "--json"])).unwrap();

        assert!(output.render().contains(r#""category":"weight""#));
    }

    #[test]
    fn renders_universal_volume_json_category() {
        let output = run(&args(&["convert", "1 gallon", "ml", "--json"])).unwrap();

        assert!(output.render().contains(r#""category":"volume""#));
    }

    #[test]
    fn renders_universal_potency_json_category() {
        let output = run(&args(&["convert", "22.4%", "mg/g", "--json"])).unwrap();

        assert!(output.render().contains(r#""category":"potency""#));
    }

    #[test]
    fn rejects_universal_mismatched_target_category() {
        assert!(run(&args(&["convert", "3.5g", "gallon"])).is_err());
    }

    #[test]
    fn rejects_universal_unparseable_input() {
        assert!(run(&args(&["convert", "abc", "g"])).is_err());
    }

    #[test]
    fn rejects_universal_missing_arguments() {
        assert!(run(&args(&["convert", "1000mg"])).is_err());
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
