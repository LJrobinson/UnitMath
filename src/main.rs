use std::{
    env,
    fmt::Write,
    fs,
    io::{self, Read},
    process,
};

use unitmath::{
    calculate_total_quantity, calculate_total_units, convert_parsed_potency, convert_parsed_volume,
    convert_parsed_weight, PotencyUnit, UnitMathError, VolumeUnit, WeightUnit,
};

fn main() {
    match run(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(output) => {
            let rendered = output.render();
            if !rendered.is_empty() {
                println!("{rendered}");
            }

            let rendered_stderr = output.render_stderr();
            if !rendered_stderr.is_empty() {
                eprintln!("{rendered_stderr}");
            }
        }
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
    precision: Option<usize>,
    include_csv_header: bool,
    csv_delimiter: CsvDelimiter,
    rendered_output: Option<String>,
    stderr_output: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Plain,
    Json,
    Csv,
}

impl CliOutput {
    fn render(&self) -> String {
        if let Some(output) = &self.rendered_output {
            return output.clone();
        }

        match self.format {
            OutputFormat::Plain => format_number(self.value, self.precision),
            OutputFormat::Json => format!(
                r#"{{"category":"{}","input":"{}","target_unit":"{}","value":{}}}"#,
                escape_json_string(&self.category),
                escape_json_string(&self.input),
                escape_json_string(&self.target_unit),
                format_number(self.value, self.precision)
            ),
            OutputFormat::Csv => render_one_shot_csv(
                &self.category,
                &self.input,
                &self.target_unit,
                self.value,
                self.precision,
                self.include_csv_header,
                self.csv_delimiter,
            ),
        }
    }

    fn render_stderr(&self) -> String {
        self.stderr_output.clone().unwrap_or_default()
    }
}

fn run(args: &[String]) -> Result<CliOutput, String> {
    run_with_stdin(args, None)
}

fn extract_precision(args: &[String]) -> Result<(Vec<String>, Option<usize>), String> {
    let precision_positions = args
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| (argument == "--precision").then_some(index))
        .collect::<Vec<usize>>();

    if precision_positions.len() > 1 {
        return Err("--precision may only be provided once".to_string());
    }

    let Some(precision_index) = precision_positions.first().copied() else {
        return Ok((args.to_vec(), None));
    };

    if precision_index + 1 >= args.len() || args[precision_index + 1].starts_with("--") {
        return Err("--precision requires a value from 0 through 12".to_string());
    }

    let parsed_precision = args[precision_index + 1]
        .parse::<i32>()
        .map_err(|_| "--precision must be an integer from 0 through 12".to_string())?;

    if !(0..=12).contains(&parsed_precision) {
        return Err("--precision must be between 0 and 12".to_string());
    }

    let cleaned_args = args
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| {
            (index != precision_index && index != precision_index + 1).then_some(argument.clone())
        })
        .collect::<Vec<String>>();

    Ok((cleaned_args, Some(parsed_precision as usize)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CsvHeaderControl {
    Include,
    Omit,
}

impl CsvHeaderControl {
    fn include_header(self) -> bool {
        matches!(self, Self::Include)
    }
}

fn extract_csv_header_control(
    args: &[String],
) -> Result<(Vec<String>, Option<CsvHeaderControl>), String> {
    let include_count = args
        .iter()
        .filter(|argument| argument.as_str() == "--include-header")
        .count();
    let omit_count = args
        .iter()
        .filter(|argument| argument.as_str() == "--no-header")
        .count();

    if include_count > 0 && omit_count > 0 {
        return Err("--include-header and --no-header cannot be used together".to_string());
    }

    if include_count > 1 {
        return Err("--include-header may only be provided once".to_string());
    }

    if omit_count > 1 {
        return Err("--no-header may only be provided once".to_string());
    }

    let control = if include_count == 1 {
        Some(CsvHeaderControl::Include)
    } else if omit_count == 1 {
        Some(CsvHeaderControl::Omit)
    } else {
        None
    };

    let cleaned_args = args
        .iter()
        .filter(|argument| {
            argument.as_str() != "--include-header" && argument.as_str() != "--no-header"
        })
        .cloned()
        .collect::<Vec<String>>();

    Ok((cleaned_args, control))
}

fn include_csv_header(control: Option<CsvHeaderControl>) -> bool {
    control
        .unwrap_or(CsvHeaderControl::Include)
        .include_header()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchRowFilter {
    All,
    ErrorsOnly,
    OkOnly,
}

impl BatchRowFilter {
    fn matches(self, result: &BatchResult) -> bool {
        match self {
            Self::All => true,
            Self::ErrorsOnly => result.status == "error",
            Self::OkOnly => result.status == "ok",
        }
    }
}

fn extract_batch_row_filter(args: &[String]) -> Result<(Vec<String>, BatchRowFilter), String> {
    let errors_only_count = args
        .iter()
        .filter(|argument| argument.as_str() == "--errors-only")
        .count();
    let ok_only_count = args
        .iter()
        .filter(|argument| argument.as_str() == "--ok-only")
        .count();

    if errors_only_count > 0 && ok_only_count > 0 {
        return Err("--errors-only and --ok-only cannot be used together".to_string());
    }

    if errors_only_count > 1 {
        return Err("--errors-only may only be provided once".to_string());
    }

    if ok_only_count > 1 {
        return Err("--ok-only may only be provided once".to_string());
    }

    let filter = if errors_only_count == 1 {
        BatchRowFilter::ErrorsOnly
    } else if ok_only_count == 1 {
        BatchRowFilter::OkOnly
    } else {
        BatchRowFilter::All
    };

    let cleaned_args = args
        .iter()
        .filter(|argument| argument.as_str() != "--errors-only" && argument.as_str() != "--ok-only")
        .cloned()
        .collect::<Vec<String>>();

    Ok((cleaned_args, filter))
}

fn extract_summary(args: &[String]) -> Result<(Vec<String>, bool), String> {
    let summary_count = args
        .iter()
        .filter(|argument| argument.as_str() == "--summary")
        .count();

    if summary_count > 1 {
        return Err("--summary may only be provided once".to_string());
    }

    let cleaned_args = args
        .iter()
        .filter(|argument| argument.as_str() != "--summary")
        .cloned()
        .collect::<Vec<String>>();

    Ok((cleaned_args, summary_count == 1))
}

fn extract_json_array(args: &[String]) -> Result<(Vec<String>, bool), String> {
    let json_array_count = args
        .iter()
        .filter(|argument| argument.as_str() == "--json-array")
        .count();

    if json_array_count > 1 {
        return Err("--json-array may only be provided once".to_string());
    }

    let cleaned_args = args
        .iter()
        .filter(|argument| argument.as_str() != "--json-array")
        .cloned()
        .collect::<Vec<String>>();

    Ok((cleaned_args, json_array_count == 1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CsvDelimiter {
    Comma,
    Tab,
    Pipe,
}

impl CsvDelimiter {
    fn as_char(self) -> char {
        match self {
            Self::Comma => ',',
            Self::Tab => '\t',
            Self::Pipe => '|',
        }
    }
}

fn parse_csv_delimiter(input: &str) -> Result<CsvDelimiter, String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "comma" => Ok(CsvDelimiter::Comma),
        "tab" => Ok(CsvDelimiter::Tab),
        "pipe" => Ok(CsvDelimiter::Pipe),
        _ => Err("--delimiter requires comma, tab, or pipe".to_string()),
    }
}

fn extract_csv_delimiter(args: &[String]) -> Result<(Vec<String>, Option<CsvDelimiter>), String> {
    let delimiter_positions = args
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| (argument == "--delimiter").then_some(index))
        .collect::<Vec<usize>>();

    if delimiter_positions.len() > 1 {
        return Err("--delimiter may only be provided once".to_string());
    }

    let Some(delimiter_index) = delimiter_positions.first().copied() else {
        return Ok((args.to_vec(), None));
    };

    if delimiter_index + 1 >= args.len() || args[delimiter_index + 1].starts_with("--") {
        return Err("--delimiter requires comma, tab, or pipe".to_string());
    }

    let delimiter = parse_csv_delimiter(&args[delimiter_index + 1])?;

    let cleaned_args = args
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| {
            (index != delimiter_index && index != delimiter_index + 1).then_some(argument.clone())
        })
        .collect::<Vec<String>>();

    Ok((cleaned_args, Some(delimiter)))
}

fn run_with_stdin(args: &[String], stdin_contents: Option<&str>) -> Result<CliOutput, String> {
    let (args, precision) = extract_precision(args)?;
    let (args, csv_header_control) = extract_csv_header_control(&args)?;
    let (args, batch_row_filter) = extract_batch_row_filter(&args)?;
    let (args, include_summary) = extract_summary(&args)?;
    let (args, use_json_array) = extract_json_array(&args)?;
    let (args, csv_delimiter) = extract_csv_delimiter(&args)?;

    if args.first().map(String::as_str) == Some("batch") {
        return run_batch_command(
            &args,
            stdin_contents,
            precision,
            csv_header_control,
            batch_row_filter,
            include_summary,
            use_json_array,
            csv_delimiter,
        );
    }

    if use_json_array {
        return Err("--json-array is only supported for batch mode".to_string());
    }

    if include_summary {
        return Err("--summary is only supported for batch mode".to_string());
    }

    if batch_row_filter != BatchRowFilter::All {
        return Err("--errors-only and --ok-only are only supported for batch mode".to_string());
    }

    if args.iter().any(|argument| argument == "--out") {
        return Err("--out is only supported for batch mode".to_string());
    }

    if args.iter().any(|argument| argument == "--input-json") {
        return Err("--input-json is only supported for batch mode".to_string());
    }

    if args.iter().any(|argument| argument == "--input-format") {
        return Err("--input-format is only supported for batch mode".to_string());
    }

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

    if csv_header_control.is_some() && format != OutputFormat::Csv {
        return Err("--include-header and --no-header require --csv".to_string());
    }

    if csv_delimiter.is_some() && format != OutputFormat::Csv {
        return Err("--delimiter requires --csv".to_string());
    }

    let include_csv_header = include_csv_header(csv_header_control);
    let csv_delimiter = csv_delimiter.unwrap_or(CsvDelimiter::Comma);

    let command_args = match format {
        OutputFormat::Json | OutputFormat::Csv => &args[..args.len() - 1],
        OutputFormat::Plain => &args,
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

    if command_args[0].trim().eq_ignore_ascii_case("package") {
        return run_package_command(
            command_args,
            format,
            precision,
            include_csv_header,
            csv_delimiter,
        );
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
        precision,
        include_csv_header,
        csv_delimiter,
        rendered_output: None,
        stderr_output: None,
    })
}

fn run_package_command(
    command_args: &[String],
    format: OutputFormat,
    precision: Option<usize>,
    include_csv_header: bool,
    csv_delimiter: CsvDelimiter,
) -> Result<CliOutput, String> {
    let subcommand = command_args[1].trim().to_ascii_lowercase();
    let expression = &command_args[2];

    let (category, target_unit, value) = match subcommand.as_str() {
        "total-units" => {
            if command_args.len() != 3 {
                return Err(
                    "package total-units expects: unitmath package total-units \"<expression>\""
                        .to_string(),
                );
            }

            let (left, right) = parse_package_expression(expression)?;

            (
                "total_units".to_string(),
                "units".to_string(),
                calculate_total_units(left, right),
            )
        }
        "total-quantity" => {
            let target_unit = if command_args.len() > 3 {
                let label = command_args[3..].join(" ");
                if label.trim().is_empty() {
                    "quantity".to_string()
                } else {
                    label.trim().to_string()
                }
            } else {
                "quantity".to_string()
            };
            let (left, right) = parse_package_expression(expression)?;

            (
                "total_quantity".to_string(),
                target_unit,
                calculate_total_quantity(left, right),
            )
        }
        _ => {
            return Err(format!("unknown package command: {}", command_args[1]));
        }
    };

    Ok(CliOutput {
        category,
        input: expression.to_string(),
        target_unit,
        value,
        format,
        precision,
        include_csv_header,
        csv_delimiter,
        rendered_output: None,
        stderr_output: None,
    })
}

#[derive(Debug, PartialEq)]
struct BatchRow {
    category: String,
    input: String,
    target_unit: String,
}

#[derive(Debug, PartialEq)]
struct BatchResult {
    category: String,
    input: String,
    target_unit: String,
    value: Option<f64>,
    status: String,
    error: Option<String>,
}

#[derive(Debug, PartialEq)]
struct BatchCommand {
    input_path: Option<String>,
    input_format: BatchInputFormat,
    format: OutputFormat,
    out_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchInputFormat {
    Csv,
    JsonLines,
}

fn run_batch_command(
    args: &[String],
    stdin_contents: Option<&str>,
    precision: Option<usize>,
    csv_header_control: Option<CsvHeaderControl>,
    batch_row_filter: BatchRowFilter,
    include_summary: bool,
    use_json_array: bool,
    csv_delimiter: Option<CsvDelimiter>,
) -> Result<CliOutput, String> {
    let command = parse_batch_command(args)?;

    if use_json_array && command.format != OutputFormat::Json {
        return Err("--json-array requires --json".to_string());
    }

    if csv_header_control.is_some() && command.format != OutputFormat::Csv {
        return Err("--include-header and --no-header require --csv".to_string());
    }

    if csv_delimiter.is_some() && command.format != OutputFormat::Csv {
        return Err("--delimiter requires --csv".to_string());
    }

    let include_csv_header = include_csv_header(csv_header_control);
    let csv_delimiter = csv_delimiter.unwrap_or(CsvDelimiter::Comma);

    let contents = match command.input_path.as_deref() {
        Some(input_path) => fs::read_to_string(input_path)
            .map_err(|error| format!("failed to read batch input file '{input_path}': {error}"))?,
        None => read_batch_stdin(stdin_contents)?,
    };
    let mut results = match command.input_format {
        BatchInputFormat::Csv => {
            let rows = parse_batch_rows(&contents)?;
            rows.iter()
                .map(process_batch_row)
                .collect::<Vec<BatchResult>>()
        }
        BatchInputFormat::JsonLines => parse_json_lines_batch_results(&contents),
    };

    let processed_count = results.len();
    let ok_count = results
        .iter()
        .filter(|result| result.status == "ok")
        .count();
    let error_count = results
        .iter()
        .filter(|result| result.status == "error")
        .count();

    results.retain(|result| batch_row_filter.matches(result));
    let emitted_count = results.len();

    let stderr_output = include_summary.then(|| {
        render_batch_summary(
            processed_count,
            ok_count,
            error_count,
            emitted_count,
            command.out_path.as_deref(),
        )
    });

    let rendered_output = match command.format {
        OutputFormat::Csv => {
            render_batch_csv(&results, precision, include_csv_header, csv_delimiter)
        }
        OutputFormat::Json if use_json_array => render_batch_json_array(&results, precision),
        OutputFormat::Json => render_batch_json_lines(&results, precision),
        OutputFormat::Plain => unreachable!("batch format is validated above"),
    };

    let rendered_output = if let Some(out_path) = command.out_path.as_deref() {
        fs::write(out_path, &rendered_output)
            .map_err(|error| format!("failed to write batch output file '{out_path}': {error}"))?;
        String::new()
    } else {
        rendered_output
    };

    Ok(CliOutput {
        category: "batch".to_string(),
        input: command
            .input_path
            .as_deref()
            .unwrap_or("<stdin>")
            .to_string(),
        target_unit: String::new(),
        value: 0.0,
        format: command.format,
        precision,
        include_csv_header,
        csv_delimiter,
        rendered_output: Some(rendered_output),
        stderr_output,
    })
}

fn parse_batch_command(args: &[String]) -> Result<BatchCommand, String> {
    if args.len() < 2 {
        return Err("batch mode requires --csv or --json".to_string());
    }

    let out_positions = args
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| (argument == "--out").then_some(index))
        .collect::<Vec<usize>>();

    if out_positions.len() > 1 {
        return Err("--out may only be provided once".to_string());
    }

    let (core_args, out_path) = if let Some(out_index) = out_positions.first().copied() {
        if out_index + 1 >= args.len() {
            return Err("--out requires a file path".to_string());
        }

        if out_index + 2 != args.len() {
            return Err("--out must be followed by the output path at the end".to_string());
        }

        let out_path = &args[out_index + 1];
        if out_path == "--csv"
            || out_path == "--json"
            || out_path == "--out"
            || out_path == "--input-json"
            || out_path == "--input-format"
            || out_path == "--include-header"
            || out_path == "--no-header"
            || out_path == "--errors-only"
            || out_path == "--ok-only"
            || out_path == "--summary"
            || out_path == "--delimiter"
            || out_path == "--json-array"
        {
            return Err("--out requires a file path".to_string());
        }

        (&args[..out_index], Some(out_path.to_string()))
    } else {
        (args, None)
    };

    let has_json = core_args.iter().any(|argument| argument == "--json");
    let has_csv = core_args.iter().any(|argument| argument == "--csv");

    if has_json && has_csv {
        return Err("--json and --csv cannot be used together".to_string());
    }

    let format = match core_args.last().map(String::as_str) {
        Some("--json") => OutputFormat::Json,
        Some("--csv") => OutputFormat::Csv,
        _ => return Err("batch mode requires --csv or --json".to_string()),
    };

    if core_args[..core_args.len() - 1]
        .iter()
        .any(|argument| argument == "--json" || argument == "--csv")
    {
        return Err("batch output format flag must appear before optional --out".to_string());
    }

    let input_args = &core_args[1..core_args.len() - 1];
    let input_json_positions = input_args
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| (argument == "--input-json").then_some(index))
        .collect::<Vec<usize>>();

    if input_json_positions.len() > 1 {
        return Err("--input-json may only be provided once".to_string());
    }

    let input_format_positions = input_args
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| (argument == "--input-format").then_some(index))
        .collect::<Vec<usize>>();

    if input_format_positions.len() > 1 {
        return Err("--input-format may only be provided once".to_string());
    }

    let explicit_input_format =
        if let Some(input_format_index) = input_format_positions.first().copied() {
            if input_format_index + 1 >= input_args.len() {
                return Err("--input-format requires csv or jsonl".to_string());
            }

            Some(parse_batch_input_format(
                &input_args[input_format_index + 1],
            )?)
        } else {
            None
        };

    let mut positional_args: Vec<&String> = Vec::new();
    let mut index = 0;
    while index < input_args.len() {
        match input_args[index].as_str() {
            "--input-json" => index += 1,
            "--input-format" => index += 2,
            argument if argument.starts_with("--") => {
                return Err("unknown batch flag".to_string());
            }
            _ => {
                positional_args.push(&input_args[index]);
                index += 1;
            }
        }
    }

    let input_path = match positional_args.len() {
        0 => None,
        1 => Some(positional_args[0].to_string()),
        _ => {
            return Err(
                "batch mode expects: unitmath batch [input] [--input-format csv|jsonl] --csv|--json [--out path]"
                    .to_string(),
            );
        }
    };

    let has_input_json = !input_json_positions.is_empty();
    let input_format = match (has_input_json, explicit_input_format) {
        (true, Some(BatchInputFormat::Csv)) => {
            return Err("--input-json cannot be used with --input-format csv".to_string());
        }
        (true, Some(BatchInputFormat::JsonLines)) | (true, None) => BatchInputFormat::JsonLines,
        (false, Some(input_format)) => input_format,
        (false, None) => match input_path.as_deref() {
            Some(input_path) => detect_batch_input_format(input_path)?,
            None => BatchInputFormat::Csv,
        },
    };

    Ok(BatchCommand {
        input_path,
        input_format,
        format,
        out_path,
    })
}

fn parse_batch_input_format(input: &str) -> Result<BatchInputFormat, String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "csv" => Ok(BatchInputFormat::Csv),
        "jsonl" => Ok(BatchInputFormat::JsonLines),
        _ => Err(format!("unsupported batch input format: {input}")),
    }
}

fn detect_batch_input_format(input_path: &str) -> Result<BatchInputFormat, String> {
    let Some(extension) = std::path::Path::new(input_path)
        .extension()
        .and_then(|extension| extension.to_str())
    else {
        return Err(unknown_batch_input_format_message(input_path));
    };

    match extension.to_ascii_lowercase().as_str() {
        "csv" => Ok(BatchInputFormat::Csv),
        "jsonl" | "ndjson" => Ok(BatchInputFormat::JsonLines),
        _ => Err(unknown_batch_input_format_message(input_path)),
    }
}

fn unknown_batch_input_format_message(input_path: &str) -> String {
    format!(
        "could not detect batch input format for '{input_path}'; use --input-format csv or --input-format jsonl"
    )
}

fn read_batch_stdin(stdin_contents: Option<&str>) -> Result<String, String> {
    if let Some(contents) = stdin_contents {
        return Ok(contents.to_string());
    }

    let mut contents = String::new();
    io::stdin()
        .read_to_string(&mut contents)
        .map_err(|error| format!("failed to read batch input from stdin: {error}"))?;

    Ok(contents)
}

fn parse_batch_rows(contents: &str) -> Result<Vec<BatchRow>, String> {
    let rows = parse_csv_rows(contents)?;
    let Some(header) = rows.first() else {
        return Err("batch input CSV is empty".to_string());
    };

    let header_names = header
        .iter()
        .map(|field| field.trim().to_ascii_lowercase())
        .collect::<Vec<String>>();

    if header_names != ["category", "input", "target_unit"] {
        return Err("batch input CSV must have headers: category,input,target_unit".to_string());
    }

    rows.iter()
        .skip(1)
        .enumerate()
        .map(|(index, row)| {
            if row.len() != 3 {
                return Err(format!(
                    "batch input row {} must have exactly 3 fields",
                    index + 2
                ));
            }

            Ok(BatchRow {
                category: row[0].trim().to_ascii_lowercase(),
                input: row[1].clone(),
                target_unit: row[2].clone(),
            })
        })
        .collect()
}

fn parse_csv_rows(contents: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = contents.chars().peekable();
    let mut in_quotes = false;

    while let Some(character) = chars.next() {
        match character {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                row.push(field);
                field = String::new();
            }
            '\n' if !in_quotes => {
                row.push(field);
                field = String::new();
                rows.push(row);
                row = Vec::new();
            }
            '\r' if !in_quotes => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(field);
                field = String::new();
                rows.push(row);
                row = Vec::new();
            }
            character => field.push(character),
        }
    }

    if in_quotes {
        return Err("unterminated quoted field in batch input CSV".to_string());
    }

    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }

    Ok(rows
        .into_iter()
        .filter(|row| row.iter().any(|field| !field.trim().is_empty()))
        .collect())
}

fn parse_json_lines_batch_results(contents: &str) -> Vec<BatchResult> {
    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| match parse_json_line_batch_row(line.trim()) {
            Ok(row) => process_batch_row(&row),
            Err(result) => result,
        })
        .collect()
}

fn parse_json_line_batch_row(line: &str) -> Result<BatchRow, BatchResult> {
    let fields = parse_json_object(line).map_err(json_line_error_result)?;

    let find_field = |name: &str| {
        fields
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value.clone()))
    };

    let category = find_field("category");
    let input = find_field("input");
    let target_unit = find_field("target_unit");

    match (category, input, target_unit) {
        (Some(category), Some(input), Some(target_unit)) => Ok(BatchRow {
            category: category.trim().to_ascii_lowercase(),
            input,
            target_unit,
        }),
        (category, input, target_unit) => Err(BatchResult {
            category: category.unwrap_or_default(),
            input: input.unwrap_or_default(),
            target_unit: target_unit.unwrap_or_default(),
            value: None,
            status: "error".to_string(),
            error: Some("JSON Lines row is missing category, input, or target_unit".to_string()),
        }),
    }
}

fn json_line_error_result(error: String) -> BatchResult {
    BatchResult {
        category: String::new(),
        input: String::new(),
        target_unit: String::new(),
        value: None,
        status: "error".to_string(),
        error: Some(error),
    }
}

fn parse_json_object(input: &str) -> Result<Vec<(String, String)>, String> {
    let characters = input.chars().collect::<Vec<char>>();
    let mut index = 0;
    let mut fields = Vec::new();

    skip_json_whitespace(&characters, &mut index);
    expect_json_character(&characters, &mut index, '{')?;
    skip_json_whitespace(&characters, &mut index);

    if consume_json_character(&characters, &mut index, '}') {
        skip_json_whitespace(&characters, &mut index);
        if index == characters.len() {
            return Ok(fields);
        }

        return Err("unexpected content after JSON object".to_string());
    }

    loop {
        skip_json_whitespace(&characters, &mut index);
        let key = parse_json_string(&characters, &mut index)?;
        skip_json_whitespace(&characters, &mut index);
        expect_json_character(&characters, &mut index, ':')?;
        skip_json_whitespace(&characters, &mut index);
        let value = parse_json_string(&characters, &mut index)?;
        fields.push((key, value));
        skip_json_whitespace(&characters, &mut index);

        if consume_json_character(&characters, &mut index, ',') {
            continue;
        }

        expect_json_character(&characters, &mut index, '}')?;
        break;
    }

    skip_json_whitespace(&characters, &mut index);
    if index != characters.len() {
        return Err("unexpected content after JSON object".to_string());
    }

    Ok(fields)
}

fn parse_json_string(characters: &[char], index: &mut usize) -> Result<String, String> {
    expect_json_character(characters, index, '"')?;
    let mut value = String::new();

    while *index < characters.len() {
        let character = characters[*index];
        *index += 1;

        match character {
            '"' => return Ok(value),
            '\\' => {
                if *index >= characters.len() {
                    return Err("unterminated JSON escape sequence".to_string());
                }

                let escaped = characters[*index];
                *index += 1;

                match escaped {
                    '"' => value.push('"'),
                    '\\' => value.push('\\'),
                    '/' => value.push('/'),
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    'b' => value.push('\u{08}'),
                    'f' => value.push('\u{0c}'),
                    _ => return Err(format!("unsupported JSON escape sequence: \\{escaped}")),
                }
            }
            character => value.push(character),
        }
    }

    Err("unterminated JSON string".to_string())
}

fn skip_json_whitespace(characters: &[char], index: &mut usize) {
    while *index < characters.len() && characters[*index].is_whitespace() {
        *index += 1;
    }
}

fn expect_json_character(
    characters: &[char],
    index: &mut usize,
    expected: char,
) -> Result<(), String> {
    if consume_json_character(characters, index, expected) {
        Ok(())
    } else {
        Err(format!("expected '{expected}'"))
    }
}

fn consume_json_character(characters: &[char], index: &mut usize, expected: char) -> bool {
    if *index < characters.len() && characters[*index] == expected {
        *index += 1;
        true
    } else {
        false
    }
}

fn process_batch_row(row: &BatchRow) -> BatchResult {
    match convert_row(&row.category, &row.input, &row.target_unit) {
        Ok((category, value)) => BatchResult {
            category,
            input: row.input.clone(),
            target_unit: row.target_unit.trim().to_string(),
            value: Some(value),
            status: "ok".to_string(),
            error: None,
        },
        Err(error) => BatchResult {
            category: row.category.clone(),
            input: row.input.clone(),
            target_unit: row.target_unit.trim().to_string(),
            value: None,
            status: "error".to_string(),
            error: Some(error),
        },
    }
}

fn convert_row(category: &str, input: &str, target: &str) -> Result<(String, f64), String> {
    match category {
        "weight" => {
            let unit = parse_target_weight_unit(target)
                .ok_or_else(|| format!("unknown weight target unit: {target}"))?;
            let value = convert_parsed_weight(input, unit)
                .map_err(|error| parse_error_message("weight", error))?;

            Ok(("weight".to_string(), value))
        }
        "volume" => {
            let unit = parse_target_volume_unit(target)
                .ok_or_else(|| format!("unknown volume target unit: {target}"))?;
            let value = convert_parsed_volume(input, unit)
                .map_err(|error| parse_error_message("volume", error))?;

            Ok(("volume".to_string(), value))
        }
        "potency" => {
            let unit = parse_target_potency_unit(target)
                .ok_or_else(|| format!("unknown potency target unit: {target}"))?;
            let value = convert_parsed_potency(input, unit)
                .map_err(|error| parse_error_message("potency", error))?;

            Ok(("potency".to_string(), value))
        }
        "convert" => infer_conversion(input, target),
        "total_units" => {
            let (left, right) = parse_package_expression(input)?;

            Ok((
                "total_units".to_string(),
                calculate_total_units(left, right),
            ))
        }
        "total_quantity" => {
            let (left, right) = parse_package_expression(input)?;

            Ok((
                "total_quantity".to_string(),
                calculate_total_quantity(left, right),
            ))
        }
        _ => Err(format!("unknown batch category: {category}")),
    }
}

fn parse_package_expression(input: &str) -> Result<(f64, f64), String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("invalid package expression: missing values".to_string());
    }

    let separators = trimmed
        .char_indices()
        .filter_map(|(index, character)| {
            matches!(character, 'x' | 'X' | '*' | ',').then_some(index)
        })
        .collect::<Vec<usize>>();

    if separators.len() != 1 {
        return Err(
            "invalid package expression: expected two numeric values separated by x, *, or comma"
                .to_string(),
        );
    }

    let separator_index = separators[0];
    let (left, right_with_separator) = trimmed.split_at(separator_index);
    let right = &right_with_separator[1..];

    let left = parse_package_value(left, "left")?;
    let right = parse_package_value(right, "right")?;

    Ok((left, right))
}

fn parse_package_value(input: &str, side: &str) -> Result<f64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(format!("invalid package expression: missing {side} value"));
    }

    trimmed
        .parse::<f64>()
        .map_err(|_| format!("invalid package expression: non-numeric {side} value"))
}

fn render_one_shot_csv(
    category: &str,
    input: &str,
    target_unit: &str,
    value: f64,
    precision: Option<usize>,
    include_header: bool,
    csv_delimiter: CsvDelimiter,
) -> String {
    let delimiter = csv_delimiter.as_char();
    let delimiter_string = delimiter.to_string();
    let header = ["category", "input", "target_unit", "value"].join(&delimiter_string);
    let row = [
        escape_csv_field(category, delimiter),
        escape_csv_field(input, delimiter),
        escape_csv_field(target_unit, delimiter),
        format_number(value, precision),
    ]
    .join(&delimiter_string);

    if include_header {
        format!("{header}\n{row}")
    } else {
        row
    }
}

fn render_batch_csv(
    results: &[BatchResult],
    precision: Option<usize>,
    include_header: bool,
    csv_delimiter: CsvDelimiter,
) -> String {
    let delimiter = csv_delimiter.as_char();
    let delimiter_string = delimiter.to_string();
    let mut output = if include_header {
        [
            "category",
            "input",
            "target_unit",
            "value",
            "status",
            "error",
        ]
        .join(&delimiter_string)
    } else {
        String::new()
    };

    for result in results {
        if !output.is_empty() {
            output.push('\n');
        }

        let value = result
            .value
            .map(|value| format_number(value, precision))
            .unwrap_or_default();
        let error = result.error.as_deref().unwrap_or("");

        output.push_str(
            &[
                escape_csv_field(&result.category, delimiter),
                escape_csv_field(&result.input, delimiter),
                escape_csv_field(&result.target_unit, delimiter),
                value,
                result.status.to_string(),
                escape_csv_field(error, delimiter),
            ]
            .join(&delimiter_string),
        );
    }

    output
}

fn render_batch_summary(
    processed_count: usize,
    ok_count: usize,
    error_count: usize,
    emitted_count: usize,
    out_path: Option<&str>,
) -> String {
    let mut summary = format!(
        "summary: processed={processed_count} ok={ok_count} errors={error_count} emitted={emitted_count}"
    );

    if let Some(out_path) = out_path {
        write!(&mut summary, " output={out_path}").unwrap();
    }

    summary
}

fn render_batch_json_lines(results: &[BatchResult], precision: Option<usize>) -> String {
    results
        .iter()
        .map(|result| render_batch_json_object(result, precision))
        .collect::<Vec<String>>()
        .join("\n")
}

fn render_batch_json_array(results: &[BatchResult], precision: Option<usize>) -> String {
    format!(
        "[{}]",
        results
            .iter()
            .map(|result| render_batch_json_object(result, precision))
            .collect::<Vec<String>>()
            .join(",")
    )
}

fn render_batch_json_object(result: &BatchResult, precision: Option<usize>) -> String {
    let value = result
        .value
        .map(|value| format_number(value, precision))
        .unwrap_or_else(|| "null".to_string());
    let error = result.error.as_ref().map_or_else(
        || "null".to_string(),
        |error| format!(r#""{}""#, escape_json_string(error)),
    );

    format!(
        r#"{{"category":"{}","input":"{}","target_unit":"{}","value":{},"status":"{}","error":{}}}"#,
        escape_json_string(&result.category),
        escape_json_string(&result.input),
        escape_json_string(&result.target_unit),
        value,
        escape_json_string(&result.status),
        error
    )
}

fn format_number(value: f64, precision: Option<usize>) -> String {
    match precision {
        Some(precision) => format!("{value:.precision$}"),
        None => value.to_string(),
    }
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

fn escape_csv_field(input: &str, delimiter: char) -> String {
    if input
        .chars()
        .any(|character| matches!(character, '"' | '\n' | '\r') || character == delimiter)
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
  unitmath package total-units "<expression>"
  unitmath package total-quantity "<expression>" [target-label]
  unitmath weight "<input>" <target-unit> --json
  unitmath volume "<input>" <target-unit> --json
  unitmath potency "<input>" <target-unit> --json
  unitmath convert "<input>" <target-unit> --json
  unitmath package total-units "<expression>" --json
  unitmath package total-quantity "<expression>" [target-label] --json
  unitmath weight "<input>" <target-unit> --csv
  unitmath volume "<input>" <target-unit> --csv
  unitmath potency "<input>" <target-unit> --csv
  unitmath convert "<input>" <target-unit> --csv
  unitmath package total-units "<expression>" --csv
  unitmath package total-quantity "<expression>" [target-label] --csv
  unitmath <command> ... --csv --delimiter comma|tab|pipe
  unitmath <command> ... --csv --no-header
  unitmath <command> ... --csv --include-header
  unitmath <command> ... --precision <digits>
  unitmath batch <input.csv> --csv
  unitmath batch <input.csv> --json
  unitmath batch <input.csv> --json --json-array
  unitmath batch <input.csv> --csv --errors-only
  unitmath batch <input.csv> --json --ok-only
  unitmath batch <input.csv> --csv --summary
  unitmath batch <input.csv> --json --summary
  unitmath batch <input.csv> --csv --delimiter comma|tab|pipe
  unitmath batch <input.csv> --csv --precision <digits>
  unitmath batch <input.csv> --json --precision <digits>
  unitmath batch <input.jsonl> --csv
  unitmath batch <input.jsonl> --json
  unitmath batch <input.ndjson> --csv
  unitmath batch --csv
  unitmath batch --json
  unitmath batch <input.csv> --input-format csv --csv
  unitmath batch <input.csv> --input-format csv --json
  unitmath batch <input.jsonl> --input-format jsonl --csv
  unitmath batch <input.jsonl> --input-format jsonl --json
  unitmath batch --input-format csv --csv
  unitmath batch --input-format jsonl --json
  unitmath batch <input.jsonl> --input-json --csv
  unitmath batch <input.csv> --csv --out <path>
  unitmath batch <input.csv> --csv --no-header --out <path>
  unitmath batch <input.csv> --json --out <path>
  unitmath batch <input.csv> --json --json-array --out <path>
  unitmath batch --csv --out <path>
  unitmath batch --json --out <path>
  unitmath batch <input.jsonl> --input-format jsonl --csv --out <path>
  unitmath batch <input.jsonl> --input-format jsonl --json --out <path>
  unitmath batch --input-format jsonl --csv --out <path>
  unitmath batch --input-format jsonl --json --out <path>

Target units:
  weight: mg, g, kg, oz, lb
  volume: ml, l, "fl oz", floz, cup, pint, quart, gallon
  potency: %, percent, mg/g, mgg"#
}

#[cfg(test)]
mod tests {
    use super::{
        escape_csv_field, escape_json_string, run, run_with_stdin, CliOutput, CsvDelimiter,
        OutputFormat,
    };
    use std::{
        env, fs,
        path::PathBuf,
        process,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static TEMP_FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);
    const JSONL_BATCH_INPUT: &str = concat!(
        r#"{"category":"weight","input":"1000mg","target_unit":"g"}"#,
        "\n",
        r#"{"category":"volume","input":"1 gallon","target_unit":"ml"}"#,
        "\n",
        r#"{"category":"potency","input":"22.4%","target_unit":"mg/g"}"#,
        "\n",
        r#"{"category":"convert","input":"8 fl oz","target_unit":"cup"}"#,
        "\n"
    );
    const JSONL_BATCH_CSV_OUTPUT: &str = concat!(
        "category,input,target_unit,value,status,error\n",
        "weight,1000mg,g,1,ok,\n",
        "volume,1 gallon,ml,3785.411784,ok,\n",
        "potency,22.4%,mg/g,224,ok,\n",
        "volume,8 fl oz,cup,1,ok,"
    );
    const JSONL_BATCH_JSON_OUTPUT: &str = concat!(
        r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
        "\n",
        r#"{"category":"volume","input":"1 gallon","target_unit":"ml","value":3785.411784,"status":"ok","error":null}"#,
        "\n",
        r#"{"category":"potency","input":"22.4%","target_unit":"mg/g","value":224,"status":"ok","error":null}"#,
        "\n",
        r#"{"category":"volume","input":"8 fl oz","target_unit":"cup","value":1,"status":"ok","error":null}"#
    );
    const JSONL_BATCH_JSON_ARRAY_OUTPUT: &str = concat!(
        "[",
        r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
        ",",
        r#"{"category":"volume","input":"1 gallon","target_unit":"ml","value":3785.411784,"status":"ok","error":null}"#,
        ",",
        r#"{"category":"potency","input":"22.4%","target_unit":"mg/g","value":224,"status":"ok","error":null}"#,
        ",",
        r#"{"category":"volume","input":"8 fl oz","target_unit":"cup","value":1,"status":"ok","error":null}"#,
        "]"
    );

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

    fn write_temp_csv(contents: &str) -> PathBuf {
        write_temp_batch_file(contents, "csv")
    }

    fn write_temp_jsonl(contents: &str) -> PathBuf {
        write_temp_batch_file(contents, "jsonl")
    }

    fn write_temp_batch_file(contents: &str, extension: &str) -> PathBuf {
        let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let file_name = if extension.is_empty() {
            format!("unitmath-batch-test-{}-{counter}", process::id())
        } else {
            format!(
                "unitmath-batch-test-{}-{counter}.{extension}",
                process::id()
            )
        };
        let path = env::temp_dir().join(file_name);

        fs::write(&path, contents).unwrap();

        path
    }

    fn temp_output_path(extension: &str) -> PathBuf {
        let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::SeqCst);

        env::temp_dir().join(format!(
            "unitmath-batch-output-test-{}-{counter}.{extension}",
            process::id()
        ))
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
            precision: None,
            include_csv_header: true,
            csv_delimiter: CsvDelimiter::Comma,
            rendered_output: None,
            stderr_output: None,
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
    fn renders_one_shot_csv_with_default_comma_delimiter() {
        let output = run(&args(&["convert", "1000mg", "g", "--csv"])).unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value\nweight,1000mg,g,1"
        );
    }

    #[test]
    fn renders_one_shot_csv_with_tab_delimiter() {
        let output = run(&args(&[
            "weight",
            "1000mg",
            "g",
            "--csv",
            "--delimiter",
            "tab",
        ]))
        .unwrap();

        assert_eq!(
            output.render(),
            "category\tinput\ttarget_unit\tvalue\nweight\t1000mg\tg\t1"
        );
    }

    #[test]
    fn renders_one_shot_csv_with_pipe_delimiter() {
        let output = run(&args(&[
            "convert",
            "1000mg",
            "g",
            "--csv",
            "--delimiter",
            "pipe",
        ]))
        .unwrap();

        assert_eq!(
            output.render(),
            "category|input|target_unit|value\nweight|1000mg|g|1"
        );
    }

    #[test]
    fn renders_one_shot_csv_output_without_header() {
        let output = run(&args(&["weight", "1000mg", "g", "--csv", "--no-header"])).unwrap();

        assert_eq!(output.render(), "weight,1000mg,g,1");
    }

    #[test]
    fn renders_one_shot_csv_output_with_explicit_header() {
        let output = run(&args(&[
            "weight",
            "1000mg",
            "g",
            "--csv",
            "--include-header",
        ]))
        .unwrap();

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
    fn calculates_package_total_units() {
        let actual = run(&args(&["package", "total-units", "2 x 12"]))
            .unwrap()
            .value;

        assert_approx_eq(actual, 24.0, 1e-12);
    }

    #[test]
    fn calculates_package_total_units_with_decimal_expression() {
        let actual = run(&args(&["package", "total-units", "1.5 x 10"]))
            .unwrap()
            .value;

        assert_approx_eq(actual, 15.0, 1e-12);
    }

    #[test]
    fn calculates_package_total_quantity() {
        let actual = run(&args(&["package", "total-quantity", "10 x 3.5"]))
            .unwrap()
            .value;

        assert_approx_eq(actual, 35.0, 1e-12);
    }

    #[test]
    fn calculates_package_total_quantity_with_label() {
        let output = run(&args(&["package", "total-quantity", "24 * 100", "mg"])).unwrap();

        assert_approx_eq(output.value, 2400.0, 1e-12);
        assert_eq!(output.target_unit, "mg");
    }

    #[test]
    fn calculates_package_total_units_with_comma_expression() {
        let actual = run(&args(&["package", "total-units", "2,12"]))
            .unwrap()
            .value;

        assert_approx_eq(actual, 24.0, 1e-12);
    }

    #[test]
    fn renders_package_total_units_json_output() {
        let output = run(&args(&["package", "total-units", "2 x 12", "--json"])).unwrap();

        assert_eq!(
            output.render(),
            r#"{"category":"total_units","input":"2 x 12","target_unit":"units","value":24}"#
        );
    }

    #[test]
    fn renders_package_total_quantity_json_output_with_label() {
        let output = run(&args(&[
            "package",
            "total-quantity",
            "24 * 100",
            "mg",
            "--json",
        ]))
        .unwrap();

        assert_eq!(
            output.render(),
            r#"{"category":"total_quantity","input":"24 * 100","target_unit":"mg","value":2400}"#
        );
    }

    #[test]
    fn renders_package_total_units_csv_output() {
        let output = run(&args(&["package", "total-units", "2,12", "--csv"])).unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value\ntotal_units,\"2,12\",units,24"
        );
    }

    #[test]
    fn renders_package_total_quantity_csv_output_with_label() {
        let output = run(&args(&[
            "package",
            "total-quantity",
            "24 * 100",
            "mg",
            "--csv",
        ]))
        .unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value\ntotal_quantity,24 * 100,mg,2400"
        );
    }

    #[test]
    fn rejects_invalid_package_expression() {
        assert!(run(&args(&["package", "total-units", "2 / 12"])).is_err());
    }

    #[test]
    fn rejects_package_json_and_csv_together() {
        assert!(run(&args(&[
            "package",
            "total-units",
            "2 x 12",
            "--json",
            "--csv"
        ]))
        .is_err());
    }

    #[test]
    fn rejects_out_on_package_command() {
        assert!(run(&args(&[
            "package",
            "total-units",
            "2 x 12",
            "--out",
            "results.csv"
        ]))
        .is_err());
    }

    #[test]
    fn renders_plain_output_with_precision() {
        let output = run(&args(&["convert", "3.5g", "oz", "--precision", "4"])).unwrap();

        assert_eq!(output.render(), "0.1235");
    }

    #[test]
    fn renders_json_output_with_precision() {
        let output = run(&args(&[
            "convert",
            "3.5g",
            "oz",
            "--json",
            "--precision",
            "4",
        ]))
        .unwrap();

        assert_eq!(
            output.render(),
            r#"{"category":"weight","input":"3.5g","target_unit":"oz","value":0.1235}"#
        );
    }

    #[test]
    fn renders_csv_output_with_precision() {
        let output = run(&args(&[
            "convert",
            "3.5g",
            "oz",
            "--csv",
            "--precision",
            "4",
        ]))
        .unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value\nweight,3.5g,oz,0.1235"
        );
    }

    #[test]
    fn renders_csv_output_with_precision_without_header() {
        let output = run(&args(&[
            "convert",
            "3.5g",
            "oz",
            "--csv",
            "--precision",
            "4",
            "--no-header",
        ]))
        .unwrap();

        assert_eq!(output.render(), "weight,3.5g,oz,0.1235");
    }

    #[test]
    fn renders_csv_output_with_precision_and_tab_delimiter() {
        let output = run(&args(&[
            "convert",
            "3.5g",
            "oz",
            "--csv",
            "--precision",
            "4",
            "--delimiter",
            "tab",
        ]))
        .unwrap();

        assert_eq!(
            output.render(),
            "category\tinput\ttarget_unit\tvalue\nweight\t3.5g\toz\t0.1235"
        );
    }

    #[test]
    fn renders_weight_output_with_precision() {
        let output = run(&args(&["weight", "1000mg", "g", "--precision", "2"])).unwrap();

        assert_eq!(output.render(), "1.00");
    }

    #[test]
    fn renders_volume_output_with_precision() {
        let output = run(&args(&["volume", "1 gallon", "ml", "--precision", "2"])).unwrap();

        assert_eq!(output.render(), "3785.41");
    }

    #[test]
    fn renders_potency_output_with_precision() {
        let output = run(&args(&["potency", "22.4%", "mg/g", "--precision", "0"])).unwrap();

        assert_eq!(output.render(), "224");
    }

    #[test]
    fn renders_package_output_with_precision() {
        let output = run(&args(&[
            "package",
            "total-units",
            "2 x 12",
            "--precision",
            "2",
        ]))
        .unwrap();

        assert_eq!(output.render(), "24.00");
    }

    #[test]
    fn renders_batch_csv_output_with_precision() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--precision", "4"]),
            Some("category,input,target_unit\nweight,3.5g,oz\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\nweight,3.5g,oz,0.1235,ok,"
        );
    }

    #[test]
    fn renders_batch_csv_output_with_tab_delimiter() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--delimiter", "tab"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category\tinput\ttarget_unit\tvalue\tstatus\terror\nweight\t1000mg\tg\t1\tok\t"
        );
    }

    #[test]
    fn renders_batch_csv_output_with_pipe_delimiter() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--delimiter", "pipe"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category|input|target_unit|value|status|error\nweight|1000mg|g|1|ok|"
        );
    }

    #[test]
    fn renders_batch_csv_output_without_header() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--no-header"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(output, "weight,1000mg,g,1,ok,");
    }

    #[test]
    fn renders_csv_output_without_header_with_pipe_delimiter() {
        let output = run(&args(&[
            "weight",
            "1000mg",
            "g",
            "--csv",
            "--no-header",
            "--delimiter",
            "pipe",
        ]))
        .unwrap();

        assert_eq!(output.render(), "weight|1000mg|g|1");
    }

    #[test]
    fn renders_batch_csv_output_with_explicit_header() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--include-header"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\nweight,1000mg,g,1,ok,"
        );
    }

    #[test]
    fn renders_batch_csv_output_with_precision_without_header() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--precision", "4", "--no-header"]),
            Some("category,input,target_unit\nweight,3.5g,oz\n"),
        )
        .unwrap()
        .render();

        assert_eq!(output, "weight,3.5g,oz,0.1235,ok,");
    }

    #[test]
    fn rejects_batch_no_header_without_csv() {
        assert!(run_with_stdin(
            &args(&["batch", "--json", "--no-header"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .is_err());
    }

    #[test]
    fn rejects_batch_include_header_without_csv() {
        assert!(run_with_stdin(
            &args(&["batch", "--json", "--include-header"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .is_err());
    }

    #[test]
    fn renders_batch_json_lines_output_with_precision() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--precision", "4"]),
            Some("category,input,target_unit\nweight,3.5g,oz\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            r#"{"category":"weight","input":"3.5g","target_unit":"oz","value":0.1235,"status":"ok","error":null}"#
        );
    }

    #[test]
    fn renders_batch_csv_output_with_summary() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--summary"]),
            Some("category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\n"),
        )
        .unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value,status,error\nweight,1000mg,g,1,ok,\nweight,10 bananas,g,,error,failed to parse weight input: unknown unit"
        );
        assert_eq!(
            output.render_stderr(),
            "summary: processed=2 ok=1 errors=1 emitted=2"
        );
    }

    #[test]
    fn renders_batch_json_lines_output_with_summary() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--json", "--summary"]),
            Some(concat!(
                r#"{"category":"weight","input":"1000mg","target_unit":"g"}"#,
                "\n",
                r#"{"category":"weight","input":"10 bananas","target_unit":"g"}"#,
                "\n"
            )),
        )
        .unwrap();

        assert_eq!(
            output.render(),
            concat!(
                r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
                "\n",
                r#"{"category":"weight","input":"10 bananas","target_unit":"g","value":null,"status":"error","error":"failed to parse weight input: unknown unit"}"#
            )
        );
        assert_eq!(
            output.render_stderr(),
            "summary: processed=2 ok=1 errors=1 emitted=2"
        );
    }

    #[test]
    fn batch_json_default_remains_json_lines() {
        let output = run_with_stdin(
            &args(&["batch", "--json"]),
            Some("category,input,target_unit\nweight,1000mg,g\nvolume,8 fl oz,cup\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            concat!(
                r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
                "\n",
                r#"{"category":"volume","input":"8 fl oz","target_unit":"cup","value":1,"status":"ok","error":null}"#
            )
        );
    }

    #[test]
    fn renders_batch_json_array_output() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--json-array"]),
            Some("category,input,target_unit\nweight,1000mg,g\nvolume,8 fl oz,cup\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            concat!(
                "[",
                r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
                ",",
                r#"{"category":"volume","input":"8 fl oz","target_unit":"cup","value":1,"status":"ok","error":null}"#,
                "]"
            )
        );
    }

    #[test]
    fn renders_batch_json_array_with_all_supported_categories() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--json-array"]),
            Some(concat!(
                "category,input,target_unit\n",
                "weight,1000mg,g\n",
                "volume,1 gallon,ml\n",
                "potency,22.4%,mg/g\n",
                "convert,8 fl oz,cup\n",
                "total_units,2 x 12,units\n",
                "total_quantity,24 * 100,mg\n"
            )),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            concat!(
                "[",
                r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
                ",",
                r#"{"category":"volume","input":"1 gallon","target_unit":"ml","value":3785.411784,"status":"ok","error":null}"#,
                ",",
                r#"{"category":"potency","input":"22.4%","target_unit":"mg/g","value":224,"status":"ok","error":null}"#,
                ",",
                r#"{"category":"volume","input":"8 fl oz","target_unit":"cup","value":1,"status":"ok","error":null}"#,
                ",",
                r#"{"category":"total_units","input":"2 x 12","target_unit":"units","value":24,"status":"ok","error":null}"#,
                ",",
                r#"{"category":"total_quantity","input":"24 * 100","target_unit":"mg","value":2400,"status":"ok","error":null}"#,
                "]"
            )
        );
    }

    #[test]
    fn renders_batch_json_array_row_level_errors() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--json-array"]),
            Some("category,input,target_unit\nweight,10 bananas,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            concat!(
                "[",
                r#"{"category":"weight","input":"10 bananas","target_unit":"g","value":null,"status":"error","error":"failed to parse weight input: unknown unit"}"#,
                "]"
            )
        );
    }

    #[test]
    fn renders_batch_json_array_errors_only() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--json-array", "--errors-only"]),
            Some("category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            concat!(
                "[",
                r#"{"category":"weight","input":"10 bananas","target_unit":"g","value":null,"status":"error","error":"failed to parse weight input: unknown unit"}"#,
                "]"
            )
        );
    }

    #[test]
    fn renders_batch_json_array_ok_only() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--json-array", "--ok-only"]),
            Some("category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            concat!(
                "[",
                r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
                "]"
            )
        );
    }

    #[test]
    fn renders_batch_json_array_zero_emitted_rows() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--json-array", "--errors-only"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(output, "[]");
    }

    #[test]
    fn renders_batch_json_array_with_precision() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--json-array", "--precision", "4"]),
            Some("category,input,target_unit\nweight,3.5g,oz\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            concat!(
                "[",
                r#"{"category":"weight","input":"3.5g","target_unit":"oz","value":0.1235,"status":"ok","error":null}"#,
                "]"
            )
        );
    }

    #[test]
    fn renders_batch_json_array_with_summary() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--json-array", "--summary"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap();

        assert_eq!(
            output.render(),
            concat!(
                "[",
                r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
                "]"
            )
        );
        assert_eq!(
            output.render_stderr(),
            "summary: processed=1 ok=1 errors=0 emitted=1"
        );
    }

    #[test]
    fn renders_stdin_batch_json_array_output() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--json-array"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            concat!(
                "[",
                r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
                "]"
            )
        );
    }

    #[test]
    fn renders_json_lines_stdin_batch_json_array_output() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--json", "--json-array"]),
            Some(JSONL_BATCH_INPUT),
        )
        .unwrap()
        .render();

        assert_eq!(output, JSONL_BATCH_JSON_ARRAY_OUTPUT);
    }

    #[test]
    fn auto_detects_jsonl_file_extension_for_json_array() {
        let path = write_temp_jsonl(JSONL_BATCH_INPUT);
        let output = run(&args(&[
            "batch",
            path.to_str().unwrap(),
            "--json",
            "--json-array",
        ]))
        .unwrap()
        .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_JSON_ARRAY_OUTPUT);
    }

    #[test]
    fn writes_batch_output_with_summary() {
        let input_path =
            write_temp_csv("category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\n");
        let output_path = temp_output_path("csv");
        let output = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--csv",
            "--out",
            output_path.to_str().unwrap(),
            "--summary",
        ]))
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(input_path).unwrap();
        fs::remove_file(&output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            "category,input,target_unit,value,status,error\nweight,1000mg,g,1,ok,\nweight,10 bananas,g,,error,failed to parse weight input: unknown unit"
        );
        assert_eq!(
            output.render_stderr(),
            format!(
                "summary: processed=2 ok=1 errors=1 emitted=2 output={}",
                output_path.to_str().unwrap()
            )
        );
    }

    #[test]
    fn writes_batch_json_array_output() {
        let input_path = write_temp_csv("category,input,target_unit\nweight,1000mg,g\n");
        let output_path = temp_output_path("json");
        let output = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--json",
            "--json-array",
            "--out",
            output_path.to_str().unwrap(),
        ]))
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            concat!(
                "[",
                r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
                "]"
            )
        );
    }

    #[test]
    fn renders_batch_csv_errors_only() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--errors-only"]),
            Some(
                "category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\nvolume,8 fl oz,cup\n",
            ),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\nweight,10 bananas,g,,error,failed to parse weight input: unknown unit"
        );
    }

    #[test]
    fn renders_filtered_batch_csv_with_pipe_delimiter_and_summary() {
        let output = run_with_stdin(
            &args(&[
                "batch",
                "--csv",
                "--ok-only",
                "--delimiter",
                "pipe",
                "--summary",
            ]),
            Some("category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\n"),
        )
        .unwrap();

        assert_eq!(
            output.render(),
            "category|input|target_unit|value|status|error\nweight|1000mg|g|1|ok|"
        );
        assert_eq!(
            output.render_stderr(),
            "summary: processed=2 ok=1 errors=1 emitted=1"
        );
    }

    #[test]
    fn renders_batch_csv_ok_only() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--ok-only"]),
            Some(
                "category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\nvolume,8 fl oz,cup\n",
            ),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\nweight,1000mg,g,1,ok,\nvolume,8 fl oz,cup,1,ok,"
        );
    }

    #[test]
    fn renders_batch_json_lines_errors_only() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--json", "--errors-only"]),
            Some(concat!(
                r#"{"category":"weight","input":"1000mg","target_unit":"g"}"#,
                "\n",
                r#"{"category":"weight","input":"10 bananas","target_unit":"g"}"#,
                "\n"
            )),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            r#"{"category":"weight","input":"10 bananas","target_unit":"g","value":null,"status":"error","error":"failed to parse weight input: unknown unit"}"#
        );
    }

    #[test]
    fn renders_batch_json_lines_ok_only() {
        let output = run_with_stdin(
            &args(&["batch", "--input-format", "jsonl", "--json", "--ok-only"]),
            Some(concat!(
                r#"{"category":"weight","input":"1000mg","target_unit":"g"}"#,
                "\n",
                r#"{"category":"weight","input":"10 bananas","target_unit":"g"}"#,
                "\n",
                r#"{"category":"volume","input":"8 fl oz","target_unit":"cup"}"#,
                "\n"
            )),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            concat!(
                r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
                "\n",
                r#"{"category":"volume","input":"8 fl oz","target_unit":"cup","value":1,"status":"ok","error":null}"#
            )
        );
    }

    #[test]
    fn renders_batch_csv_errors_only_without_header() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--errors-only", "--no-header"]),
            Some("category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "weight,10 bananas,g,,error,failed to parse weight input: unknown unit"
        );
    }

    #[test]
    fn renders_batch_csv_errors_only_zero_matches_with_header() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--errors-only"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(output, "category,input,target_unit,value,status,error");
    }

    #[test]
    fn renders_batch_csv_errors_only_zero_matches_without_header() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--errors-only", "--no-header"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(output, "");
    }

    #[test]
    fn renders_batch_json_lines_errors_only_zero_matches_as_empty_string() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--errors-only"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(output, "");
    }

    #[test]
    fn summarizes_errors_only_before_and_after_filtering() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--errors-only", "--summary"]),
            Some(
                "category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\nvolume,8 fl oz,cup\n",
            ),
        )
        .unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value,status,error\nweight,10 bananas,g,,error,failed to parse weight input: unknown unit"
        );
        assert_eq!(
            output.render_stderr(),
            "summary: processed=3 ok=2 errors=1 emitted=1"
        );
    }

    #[test]
    fn summarizes_ok_only_before_and_after_filtering() {
        let output = run_with_stdin(
            &args(&["batch", "--json", "--ok-only", "--summary"]),
            Some("category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\n"),
        )
        .unwrap();

        assert_eq!(
            output.render(),
            r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#
        );
        assert_eq!(
            output.render_stderr(),
            "summary: processed=2 ok=1 errors=1 emitted=1"
        );
    }

    #[test]
    fn summarizes_zero_rows() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--summary"]),
            Some("category,input,target_unit\n"),
        )
        .unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value,status,error"
        );
        assert_eq!(
            output.render_stderr(),
            "summary: processed=0 ok=0 errors=0 emitted=0"
        );
    }

    #[test]
    fn summarizes_precision_output() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--precision", "4", "--summary"]),
            Some("category,input,target_unit\nweight,3.5g,oz\n"),
        )
        .unwrap();

        assert_eq!(
            output.render(),
            "category,input,target_unit,value,status,error\nweight,3.5g,oz,0.1235,ok,"
        );
        assert_eq!(
            output.render_stderr(),
            "summary: processed=1 ok=1 errors=0 emitted=1"
        );
    }

    #[test]
    fn summarizes_no_header_output() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--no-header", "--summary"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap();

        assert_eq!(output.render(), "weight,1000mg,g,1,ok,");
        assert_eq!(
            output.render_stderr(),
            "summary: processed=1 ok=1 errors=0 emitted=1"
        );
    }

    #[test]
    fn filters_batch_output_with_precision() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--ok-only", "--precision", "4"]),
            Some("category,input,target_unit\nweight,3.5g,oz\nweight,10 bananas,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\nweight,3.5g,oz,0.1235,ok,"
        );
    }

    #[test]
    fn rejects_conflicting_batch_row_filters() {
        assert!(run_with_stdin(
            &args(&["batch", "--csv", "--errors-only", "--ok-only"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .is_err());
    }

    #[test]
    fn writes_batch_output_with_precision() {
        let input_path = write_temp_csv("category,input,target_unit\nweight,1000mg,g\n");
        let output_path = temp_output_path("csv");
        let output = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--csv",
            "--precision",
            "2",
            "--out",
            output_path.to_str().unwrap(),
        ]))
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            "category,input,target_unit,value,status,error\nweight,1000mg,g,1.00,ok,"
        );
    }

    #[test]
    fn writes_batch_csv_output_without_header() {
        let input_path = write_temp_csv("category,input,target_unit\nweight,1000mg,g\n");
        let output_path = temp_output_path("csv");
        let output = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--csv",
            "--out",
            output_path.to_str().unwrap(),
            "--no-header",
        ]))
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(written, "weight,1000mg,g,1,ok,");
    }

    #[test]
    fn writes_batch_csv_output_with_pipe_delimiter() {
        let input_path = write_temp_csv("category,input,target_unit\nweight,1000mg,g\n");
        let output_path = temp_output_path("csv");
        let output = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--csv",
            "--out",
            output_path.to_str().unwrap(),
            "--delimiter",
            "pipe",
        ]))
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            "category|input|target_unit|value|status|error\nweight|1000mg|g|1|ok|"
        );
    }

    #[test]
    fn writes_batch_errors_only_output() {
        let input_path =
            write_temp_csv("category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\n");
        let output_path = temp_output_path("csv");
        let output = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--csv",
            "--out",
            output_path.to_str().unwrap(),
            "--errors-only",
        ]))
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            "category,input,target_unit,value,status,error\nweight,10 bananas,g,,error,failed to parse weight input: unknown unit"
        );
    }

    #[test]
    fn writes_batch_ok_only_output() {
        let input_path =
            write_temp_csv("category,input,target_unit\nweight,1000mg,g\nweight,10 bananas,g\n");
        let output_path = temp_output_path("jsonl");
        let output = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--json",
            "--out",
            output_path.to_str().unwrap(),
            "--ok-only",
        ]))
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#
        );
    }

    #[test]
    fn rejects_missing_precision_value() {
        assert!(run(&args(&["weight", "1000mg", "g", "--precision"])).is_err());
    }

    #[test]
    fn rejects_non_integer_precision_value() {
        assert!(run(&args(&["weight", "1000mg", "g", "--precision", "two"])).is_err());
    }

    #[test]
    fn rejects_negative_precision_value() {
        assert!(run(&args(&["weight", "1000mg", "g", "--precision", "-1"])).is_err());
    }

    #[test]
    fn rejects_precision_greater_than_twelve() {
        assert!(run(&args(&["weight", "1000mg", "g", "--precision", "13"])).is_err());
    }

    #[test]
    fn rejects_duplicate_precision() {
        assert!(run(&args(&[
            "weight",
            "1000mg",
            "g",
            "--precision",
            "2",
            "--precision",
            "3"
        ]))
        .is_err());
    }

    #[test]
    fn escapes_csv_fields_with_commas() {
        let output = CliOutput {
            category: "weight".to_string(),
            input: "3,5g".to_string(),
            target_unit: "oz".to_string(),
            value: 1.0,
            format: OutputFormat::Csv,
            precision: None,
            include_csv_header: true,
            csv_delimiter: CsvDelimiter::Comma,
            rendered_output: None,
            stderr_output: None,
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
            precision: None,
            include_csv_header: true,
            csv_delimiter: CsvDelimiter::Comma,
            rendered_output: None,
            stderr_output: None,
        };

        assert_eq!(
            output.render(),
            "category,input,target_unit,value\nweight,\"3.5\"\"g\",oz,1"
        );
        assert_eq!(escape_csv_field("line\nbreak", ','), "\"line\nbreak\"");
    }

    #[test]
    fn quotes_fields_containing_active_delimiter() {
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--delimiter", "pipe"]),
            Some("category,input,target_unit\nweight,10|bananas,g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category|input|target_unit|value|status|error\nweight|\"10|bananas\"|g||error|failed to parse weight input: invalid numeric value"
        );
    }

    #[test]
    fn escapes_quotes_in_csv_fields() {
        let output = run_with_stdin(
            &args(&["batch", "--csv"]),
            Some("category,input,target_unit\nweight,\"10 \"\"bananas\"\"\",g\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\nweight,\"10 \"\"bananas\"\"\",g,,error,failed to parse weight input: invalid numeric value"
        );
    }

    #[test]
    fn rejects_json_and_csv_together() {
        assert!(run(&args(&["weight", "1000mg", "g", "--json", "--csv"])).is_err());
    }

    #[test]
    fn rejects_no_header_without_csv() {
        assert!(run(&args(&["weight", "1000mg", "g", "--no-header"])).is_err());
    }

    #[test]
    fn rejects_errors_only_on_non_batch_command() {
        assert!(run(&args(&["weight", "1000mg", "g", "--errors-only"])).is_err());
    }

    #[test]
    fn rejects_ok_only_on_non_batch_command() {
        assert!(run(&args(&["weight", "1000mg", "g", "--ok-only"])).is_err());
    }

    #[test]
    fn rejects_summary_on_non_batch_command() {
        assert!(run(&args(&["weight", "1000mg", "g", "--summary"])).is_err());
    }

    #[test]
    fn rejects_json_array_without_json() {
        assert!(run_with_stdin(
            &args(&["batch", "--json-array"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .is_err());
    }

    #[test]
    fn rejects_json_array_with_csv() {
        assert!(run_with_stdin(
            &args(&["batch", "--csv", "--json-array"]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .is_err());
    }

    #[test]
    fn rejects_json_array_on_non_batch_command() {
        assert!(run(&args(&["weight", "1000mg", "g", "--json", "--json-array"])).is_err());
    }

    #[test]
    fn rejects_delimiter_without_csv() {
        assert!(run(&args(&["weight", "1000mg", "g", "--delimiter", "tab"])).is_err());
    }

    #[test]
    fn rejects_delimiter_with_json() {
        assert!(run(&args(&[
            "weight",
            "1000mg",
            "g",
            "--json",
            "--delimiter",
            "tab"
        ]))
        .is_err());
    }

    #[test]
    fn rejects_missing_delimiter_value() {
        assert!(run(&args(&["weight", "1000mg", "g", "--csv", "--delimiter"])).is_err());
    }

    #[test]
    fn rejects_unsupported_delimiter_value() {
        assert!(run(&args(&[
            "weight",
            "1000mg",
            "g",
            "--csv",
            "--delimiter",
            "semicolon"
        ]))
        .is_err());
    }

    #[test]
    fn rejects_duplicate_delimiter() {
        assert!(run(&args(&[
            "weight",
            "1000mg",
            "g",
            "--csv",
            "--delimiter",
            "tab",
            "--delimiter",
            "pipe"
        ]))
        .is_err());
    }

    #[test]
    fn rejects_include_header_without_csv() {
        assert!(run(&args(&["weight", "1000mg", "g", "--include-header"])).is_err());
    }

    #[test]
    fn rejects_conflicting_csv_header_flags() {
        assert!(run(&args(&[
            "weight",
            "1000mg",
            "g",
            "--csv",
            "--include-header",
            "--no-header"
        ]))
        .is_err());
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
    fn renders_batch_csv_output() {
        let path = write_temp_csv(
            "category,input,target_unit\nweight,1000mg,g\nvolume,1 gallon,ml\npotency,22.4%,mg/g\nconvert,8 fl oz,cup\n",
        );
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\nweight,1000mg,g,1,ok,\nvolume,1 gallon,ml,3785.411784,ok,\npotency,22.4%,mg/g,224,ok,\nvolume,8 fl oz,cup,1,ok,"
        );
    }

    #[test]
    fn renders_stdin_batch_csv_output() {
        let output = run_with_stdin(
            &args(&["batch", "--csv"]),
            Some(
                "category,input,target_unit\nweight,1000mg,g\nvolume,1 gallon,ml\npotency,22.4%,mg/g\nconvert,8 fl oz,cup\n",
            ),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\nweight,1000mg,g,1,ok,\nvolume,1 gallon,ml,3785.411784,ok,\npotency,22.4%,mg/g,224,ok,\nvolume,8 fl oz,cup,1,ok,"
        );
    }

    #[test]
    fn renders_batch_json_lines_output() {
        let path = write_temp_csv(
            "category,input,target_unit\nweight,1000mg,g\nvolume,1 gallon,ml\npotency,22.4%,mg/g\nconvert,8 fl oz,cup\n",
        );
        let output = run(&args(&["batch", path.to_str().unwrap(), "--json"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(
            output,
            concat!(
                r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
                "\n",
                r#"{"category":"volume","input":"1 gallon","target_unit":"ml","value":3785.411784,"status":"ok","error":null}"#,
                "\n",
                r#"{"category":"potency","input":"22.4%","target_unit":"mg/g","value":224,"status":"ok","error":null}"#,
                "\n",
                r#"{"category":"volume","input":"8 fl oz","target_unit":"cup","value":1,"status":"ok","error":null}"#
            )
        );
    }

    #[test]
    fn renders_stdin_batch_json_lines_output() {
        let output = run_with_stdin(
            &args(&["batch", "--json"]),
            Some(
                "category,input,target_unit\nweight,1000mg,g\nvolume,1 gallon,ml\npotency,22.4%,mg/g\nconvert,8 fl oz,cup\n",
            ),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            concat!(
                r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#,
                "\n",
                r#"{"category":"volume","input":"1 gallon","target_unit":"ml","value":3785.411784,"status":"ok","error":null}"#,
                "\n",
                r#"{"category":"potency","input":"22.4%","target_unit":"mg/g","value":224,"status":"ok","error":null}"#,
                "\n",
                r#"{"category":"volume","input":"8 fl oz","target_unit":"cup","value":1,"status":"ok","error":null}"#
            )
        );
    }

    #[test]
    fn renders_csv_batch_total_units_row() {
        let path = write_temp_csv("category,input,target_unit\ntotal_units,2 x 12,units\n");
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\ntotal_units,2 x 12,units,24,ok,"
        );
    }

    #[test]
    fn renders_csv_batch_total_quantity_row() {
        let path = write_temp_csv("category,input,target_unit\ntotal_quantity,10 x 3.5,grams\n");
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\ntotal_quantity,10 x 3.5,grams,35,ok,"
        );
    }

    #[test]
    fn renders_csv_batch_total_quantity_row_with_star_expression() {
        let path = write_temp_csv("category,input,target_unit\ntotal_quantity,24 * 100,mg\n");
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\ntotal_quantity,24 * 100,mg,2400,ok,"
        );
    }

    #[test]
    fn renders_csv_batch_total_units_row_with_decimal_expression() {
        let path = write_temp_csv("category,input,target_unit\ntotal_units,1.5 x 10,units\n");
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\ntotal_units,1.5 x 10,units,15,ok,"
        );
    }

    #[test]
    fn renders_csv_batch_package_row_with_comma_expression() {
        let path = write_temp_csv("category,input,target_unit\ntotal_units,\"2,12\",units\n");
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\ntotal_units,\"2,12\",units,24,ok,"
        );
    }

    #[test]
    fn renders_json_lines_batch_total_units_row() {
        let output = run_with_stdin(
            &args(&["batch", "--input-format", "jsonl", "--csv"]),
            Some(r#"{"category":"total_units","input":"2 x 12","target_unit":"units"}"#),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\ntotal_units,2 x 12,units,24,ok,"
        );
    }

    #[test]
    fn renders_json_lines_batch_total_quantity_row() {
        let output = run_with_stdin(
            &args(&["batch", "--input-format", "jsonl", "--json"]),
            Some(r#"{"category":"total_quantity","input":"10 x 3.5","target_unit":"grams"}"#),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            r#"{"category":"total_quantity","input":"10 x 3.5","target_unit":"grams","value":35,"status":"ok","error":null}"#
        );
    }

    #[test]
    fn renders_stdin_batch_package_row() {
        let output = run_with_stdin(
            &args(&["batch", "--csv"]),
            Some("category,input,target_unit\ntotal_units,2 * 12,units\n"),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\ntotal_units,2 * 12,units,24,ok,"
        );
    }

    #[test]
    fn writes_package_batch_output() {
        let input_path = write_temp_csv("category,input,target_unit\ntotal_quantity,24 * 100,mg\n");
        let output_path = temp_output_path("csv");
        let output = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--csv",
            "--out",
            output_path.to_str().unwrap(),
        ]))
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            "category,input,target_unit,value,status,error\ntotal_quantity,24 * 100,mg,2400,ok,"
        );
    }

    #[test]
    fn renders_package_row_level_errors() {
        let output = run_with_stdin(
            &args(&["batch", "--csv"]),
            Some(
                "category,input,target_unit\ntotal_units,2 x,units\ntotal_quantity,two x 12,grams\ntotal_units,2 / 12,units\nweight,1000mg,g\n",
            ),
        )
        .unwrap()
        .render();

        assert!(output.contains(
            "total_units,2 x,units,,error,invalid package expression: missing right value"
        ));
        assert!(output.contains("total_quantity,two x 12,grams,,error,invalid package expression: non-numeric left value"));
        assert!(output.contains("total_units,2 / 12,units,,error,\"invalid package expression: expected two numeric values separated by x, *, or comma\""));
        assert!(output.contains("weight,1000mg,g,1,ok,"));
    }

    #[test]
    fn renders_csv_batch_csv_output_with_input_format_csv() {
        let path = write_temp_csv(
            "category,input,target_unit\nweight,1000mg,g\nvolume,1 gallon,ml\npotency,22.4%,mg/g\nconvert,8 fl oz,cup\n",
        );
        let output = run(&args(&[
            "batch",
            path.to_str().unwrap(),
            "--input-format",
            "csv",
            "--csv",
        ]))
        .unwrap()
        .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn renders_csv_batch_json_output_with_input_format_csv() {
        let path = write_temp_csv(
            "category,input,target_unit\nweight,1000mg,g\nvolume,1 gallon,ml\npotency,22.4%,mg/g\nconvert,8 fl oz,cup\n",
        );
        let output = run(&args(&[
            "batch",
            path.to_str().unwrap(),
            "--input-format",
            "csv",
            "--json",
        ]))
        .unwrap()
        .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_JSON_OUTPUT);
    }

    #[test]
    fn renders_json_lines_batch_csv_output_with_input_format_jsonl() {
        let path = write_temp_jsonl(JSONL_BATCH_INPUT);
        let output = run(&args(&[
            "batch",
            path.to_str().unwrap(),
            "--input-format",
            "jsonl",
            "--csv",
        ]))
        .unwrap()
        .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn renders_json_lines_batch_json_output_with_input_format_jsonl() {
        let path = write_temp_jsonl(JSONL_BATCH_INPUT);
        let output = run(&args(&[
            "batch",
            path.to_str().unwrap(),
            "--input-format",
            "jsonl",
            "--json",
        ]))
        .unwrap()
        .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_JSON_OUTPUT);
    }

    #[test]
    fn renders_stdin_csv_batch_with_input_format_csv() {
        let output = run_with_stdin(
            &args(&["batch", "--input-format", "csv", "--csv"]),
            Some(
                "category,input,target_unit\nweight,1000mg,g\nvolume,1 gallon,ml\npotency,22.4%,mg/g\nconvert,8 fl oz,cup\n",
            ),
        )
        .unwrap()
        .render();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn renders_stdin_json_lines_batch_with_input_format_jsonl() {
        let output = run_with_stdin(
            &args(&["batch", "--input-format", "jsonl", "--json"]),
            Some(JSONL_BATCH_INPUT),
        )
        .unwrap()
        .render();

        assert_eq!(output, JSONL_BATCH_JSON_OUTPUT);
    }

    #[test]
    fn auto_detects_csv_file_extension() {
        let path = write_temp_csv(
            "category,input,target_unit\nweight,1000mg,g\nvolume,1 gallon,ml\npotency,22.4%,mg/g\nconvert,8 fl oz,cup\n",
        );
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn auto_detects_csv_file_extension_case_insensitively() {
        let path = write_temp_batch_file(
            "category,input,target_unit\nweight,1000mg,g\nvolume,1 gallon,ml\npotency,22.4%,mg/g\nconvert,8 fl oz,cup\n",
            "CSV",
        );
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn auto_detects_jsonl_file_extension() {
        let path = write_temp_jsonl(JSONL_BATCH_INPUT);
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn auto_detects_jsonl_file_extension_case_insensitively() {
        let path = write_temp_batch_file(JSONL_BATCH_INPUT, "JSONL");
        let output = run(&args(&["batch", path.to_str().unwrap(), "--json"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_JSON_OUTPUT);
    }

    #[test]
    fn auto_detects_ndjson_file_extension() {
        let path = write_temp_batch_file(JSONL_BATCH_INPUT, "ndjson");
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn explicit_input_format_jsonl_overrides_csv_extension() {
        let path = write_temp_batch_file(JSONL_BATCH_INPUT, "csv");
        let output = run(&args(&[
            "batch",
            path.to_str().unwrap(),
            "--input-format",
            "jsonl",
            "--csv",
        ]))
        .unwrap()
        .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn explicit_input_format_csv_overrides_jsonl_extension() {
        let path = write_temp_batch_file(
            "category,input,target_unit\nweight,1000mg,g\nvolume,1 gallon,ml\npotency,22.4%,mg/g\nconvert,8 fl oz,cup\n",
            "jsonl",
        );
        let output = run(&args(&[
            "batch",
            path.to_str().unwrap(),
            "--input-format",
            "csv",
            "--json",
        ]))
        .unwrap()
        .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_JSON_OUTPUT);
    }

    #[test]
    fn input_json_overrides_csv_extension() {
        let path = write_temp_batch_file(JSONL_BATCH_INPUT, "csv");
        let output = run(&args(&[
            "batch",
            path.to_str().unwrap(),
            "--input-json",
            "--csv",
        ]))
        .unwrap()
        .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn rejects_unknown_extension_without_input_format() {
        let path = write_temp_batch_file("category,input,target_unit\nweight,1000mg,g\n", "txt");
        let result = run(&args(&["batch", path.to_str().unwrap(), "--csv"]));

        fs::remove_file(path).unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn rejects_no_extension_without_input_format() {
        let path = write_temp_batch_file("category,input,target_unit\nweight,1000mg,g\n", "");
        let result = run(&args(&["batch", path.to_str().unwrap(), "--csv"]));

        fs::remove_file(path).unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn stdin_json_lines_still_works_with_input_format_jsonl() {
        let output = run_with_stdin(
            &args(&["batch", "--input-format", "jsonl", "--csv"]),
            Some(JSONL_BATCH_INPUT),
        )
        .unwrap()
        .render();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn renders_json_lines_file_batch_csv_output() {
        let path = write_temp_jsonl(JSONL_BATCH_INPUT);
        let output = run(&args(&[
            "batch",
            path.to_str().unwrap(),
            "--input-json",
            "--csv",
        ]))
        .unwrap()
        .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn renders_json_lines_file_batch_json_lines_output() {
        let path = write_temp_jsonl(JSONL_BATCH_INPUT);
        let output = run(&args(&[
            "batch",
            path.to_str().unwrap(),
            "--input-json",
            "--json",
        ]))
        .unwrap()
        .render();

        fs::remove_file(path).unwrap();

        assert_eq!(output, JSONL_BATCH_JSON_OUTPUT);
    }

    #[test]
    fn renders_json_lines_stdin_batch_csv_output() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--csv"]),
            Some(JSONL_BATCH_INPUT),
        )
        .unwrap()
        .render();

        assert_eq!(output, JSONL_BATCH_CSV_OUTPUT);
    }

    #[test]
    fn renders_json_lines_stdin_batch_json_lines_output() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--json"]),
            Some(JSONL_BATCH_INPUT),
        )
        .unwrap()
        .render();

        assert_eq!(output, JSONL_BATCH_JSON_OUTPUT);
    }

    #[test]
    fn input_json_compatibility_alias_still_works() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--csv"]),
            Some(r#"{"category":"weight","input":"1000mg","target_unit":"g"}"#),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\nweight,1000mg,g,1,ok,"
        );
    }

    #[test]
    fn allows_input_json_with_input_format_jsonl() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--input-format", "jsonl", "--csv"]),
            Some(r#"{"category":"weight","input":"1000mg","target_unit":"g"}"#),
        )
        .unwrap()
        .render();

        assert_eq!(
            output,
            "category,input,target_unit,value,status,error\nweight,1000mg,g,1,ok,"
        );
    }

    #[test]
    fn rejects_input_json_with_input_format_csv() {
        assert!(run_with_stdin(
            &args(&["batch", "--input-json", "--input-format", "csv", "--csv"]),
            Some(r#"{"category":"weight","input":"1000mg","target_unit":"g"}"#),
        )
        .is_err());
    }

    #[test]
    fn rejects_missing_input_format_value() {
        assert!(run(&args(&["batch", "--input-format", "--csv"])).is_err());
    }

    #[test]
    fn rejects_unsupported_input_format_value() {
        assert!(run(&args(&["batch", "--input-format", "json", "--csv"])).is_err());
    }

    #[test]
    fn rejects_duplicate_input_format() {
        assert!(run(&args(&[
            "batch",
            "--input-format",
            "csv",
            "--input-format",
            "jsonl",
            "--csv",
        ]))
        .is_err());
    }

    #[test]
    fn rejects_input_format_on_non_batch_command() {
        assert!(run(&args(&["weight", "1000mg", "g", "--input-format", "csv"])).is_err());
    }

    #[test]
    fn writes_json_lines_batch_output() {
        let input_path =
            write_temp_jsonl(r#"{"category":"weight","input":"1000mg","target_unit":"g"}"#);
        let output_path = temp_output_path("jsonl");
        let output = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--input-json",
            "--json",
            "--out",
            output_path.to_str().unwrap(),
        ]))
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#
        );
    }

    #[test]
    fn renders_json_lines_row_level_conversion_errors() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--csv"]),
            Some(r#"{"category":"weight","input":"10 bananas","target_unit":"g"}"#),
        )
        .unwrap()
        .render();

        assert!(output
            .contains("weight,10 bananas,g,,error,failed to parse weight input: unknown unit"));
    }

    #[test]
    fn renders_json_lines_malformed_rows_as_row_level_errors() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--csv"]),
            Some(concat!(
                "not json\n",
                r#"{"category":"weight","input":"1000mg","target_unit":"g"}"#,
                "\n"
            )),
        )
        .unwrap()
        .render();

        assert!(output.contains(",,,,error,expected '{'"));
        assert!(output.contains("weight,1000mg,g,1,ok,"));
    }

    #[test]
    fn renders_json_lines_missing_required_fields_as_row_level_errors() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--csv"]),
            Some(r#"{"category":"weight","input":"1000mg"}"#),
        )
        .unwrap()
        .render();

        assert!(output.contains(
            "weight,1000mg,,,error,\"JSON Lines row is missing category, input, or target_unit\""
        ));
    }

    #[test]
    fn parses_escaped_json_lines_string_fields() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--json"]),
            Some(concat!(
                r#"{"category":"weight","input":"10 \"bananas\"","target_unit":"g"}"#,
                "\n",
                r#"{"category":"weight","input":"10 \\bananas","target_unit":"g"}"#,
                "\n"
            )),
        )
        .unwrap()
        .render();

        assert!(output.contains(r#""input":"10 \"bananas\""#));
        assert!(output.contains(r#""input":"10 \\bananas"#));
    }

    #[test]
    fn parses_json_lines_string_fields_containing_commas() {
        let output = run_with_stdin(
            &args(&["batch", "--input-json", "--csv"]),
            Some(r#"{"category":"weight","input":"10, bananas","target_unit":"g"}"#),
        )
        .unwrap()
        .render();

        assert!(output.contains("weight,\"10, bananas\",g,,error"));
    }

    #[test]
    fn rejects_input_json_on_non_batch_command() {
        assert!(run(&args(&["weight", "1000mg", "g", "--input-json"])).is_err());
    }

    #[test]
    fn renders_batch_row_level_errors() {
        let path = write_temp_csv(
            "category,input,target_unit\nweight,10 bananas,g\nvolume,1000ml,bucket\nconvert,abc,g\n",
        );
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert!(output
            .contains("weight,10 bananas,g,,error,failed to parse weight input: unknown unit"));
        assert!(output.contains("volume,1000ml,bucket,,error,unknown volume target unit: bucket"));
        assert!(output.contains("convert,abc,g,,error,could not infer conversion category"));
    }

    #[test]
    fn renders_stdin_batch_row_level_errors() {
        let output = run_with_stdin(
            &args(&["batch", "--csv"]),
            Some(
                "category,input,target_unit\nweight,10 bananas,g\nvolume,1000ml,bucket\nconvert,abc,g\n",
            ),
        )
        .unwrap()
        .render();

        assert!(output
            .contains("weight,10 bananas,g,,error,failed to parse weight input: unknown unit"));
        assert!(output.contains("volume,1000ml,bucket,,error,unknown volume target unit: bucket"));
        assert!(output.contains("convert,abc,g,,error,could not infer conversion category"));
    }

    #[test]
    fn writes_batch_file_csv_output() {
        let input_path = write_temp_csv("category,input,target_unit\nweight,1000mg,g\n");
        let output_path = temp_output_path("csv");
        let output = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--csv",
            "--out",
            output_path.to_str().unwrap(),
        ]))
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            "category,input,target_unit,value,status,error\nweight,1000mg,g,1,ok,"
        );
    }

    #[test]
    fn writes_batch_file_json_lines_output() {
        let input_path = write_temp_csv("category,input,target_unit\nweight,1000mg,g\n");
        let output_path = temp_output_path("jsonl");
        let output = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--json",
            "--out",
            output_path.to_str().unwrap(),
        ]))
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#
        );
    }

    #[test]
    fn writes_stdin_batch_csv_output() {
        let output_path = temp_output_path("csv");
        let output = run_with_stdin(
            &args(&["batch", "--csv", "--out", output_path.to_str().unwrap()]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            "category,input,target_unit,value,status,error\nweight,1000mg,g,1,ok,"
        );
    }

    #[test]
    fn writes_stdin_batch_json_lines_output() {
        let output_path = temp_output_path("jsonl");
        let output = run_with_stdin(
            &args(&["batch", "--json", "--out", output_path.to_str().unwrap()]),
            Some("category,input,target_unit\nweight,1000mg,g\n"),
        )
        .unwrap();
        let written = fs::read_to_string(&output_path).unwrap();

        fs::remove_file(output_path).unwrap();

        assert_eq!(output.render(), "");
        assert_eq!(
            written,
            r#"{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}"#
        );
    }

    #[test]
    fn rejects_batch_out_without_path() {
        assert!(run(&args(&["batch", "--csv", "--out"])).is_err());
    }

    #[test]
    fn rejects_duplicate_batch_out() {
        assert!(run(&args(&[
            "batch", "--csv", "--out", "one.csv", "--out", "two.csv"
        ]))
        .is_err());
    }

    #[test]
    fn rejects_out_on_non_batch_command() {
        assert!(run(&args(&["weight", "1000mg", "g", "--out", "results.csv"])).is_err());
    }

    #[test]
    fn rejects_invalid_batch_output_path() {
        let input_path = write_temp_csv("category,input,target_unit\nweight,1000mg,g\n");
        let invalid_output_path = env::temp_dir()
            .join("unitmath-missing-output-dir")
            .join("results.csv");

        let result = run(&args(&[
            "batch",
            input_path.to_str().unwrap(),
            "--csv",
            "--out",
            invalid_output_path.to_str().unwrap(),
        ]));

        fs::remove_file(input_path).unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn rejects_batch_missing_output_format() {
        assert!(run(&args(&["batch"])).is_err());
        assert!(run(&args(&["batch", "input.csv"])).is_err());
    }

    #[test]
    fn rejects_invalid_batch_file_path() {
        assert!(run(&args(&[
            "batch",
            "missing-unitmath-batch-file.csv",
            "--csv"
        ]))
        .is_err());
    }

    #[test]
    fn parses_quoted_batch_field_containing_comma() {
        let path = write_temp_csv("category,input,target_unit\nweight,\"10, bananas\",g\n");
        let output = run(&args(&["batch", path.to_str().unwrap(), "--csv"]))
            .unwrap()
            .render();

        fs::remove_file(path).unwrap();

        assert!(output.contains("weight,\"10, bananas\",g,,error"));
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
