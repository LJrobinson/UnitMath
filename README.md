# UnitMath

UnitMath is a small Rust library and dependency-free CLI for unit conversions, parsing, and package math. The library focuses on core typed conversions and parsers; the CLI adds convenient one-shot commands, package math commands, and batch processing for CSV and JSON Lines data.

## Current Scope

UnitMath currently supports:

- Weight conversions and parsing
- US liquid volume conversions and parsing
- Potency conversions and parsing
- Simple package math helpers
- One-shot CLI conversion commands
- Batch CLI processing from CSV, JSON Lines, files, and stdin
- Machine-readable CLI output as JSON, CSV-style rows, JSON Lines, or JSON arrays

UnitMath intentionally keeps the public library API focused and lightweight. Higher-level features such as universal conversion, package expression parsing, batch processing, CSV/JSON formatting, and input-format handling are currently CLI-layer features, while trait-based quantity abstractions are intentionally deferred.

## Supported Units

### Weight

- Milligram: `mg`
- Gram: `g`, `gram`, `grams`
- Kilogram: `kg`
- Ounce: `oz`, `ounce`, `ounces`
- Pound: `lb`, `lbs`, `pound`, `pounds`

Weight conversions use grams as the canonical base unit internally.

### Volume

- Milliliter: `ml`, `milliliter`, `milliliters`
- Liter: `l`, `liter`, `liters`
- US fluid ounce: `fl oz`, `floz`, `fluid ounce`, `fluid ounces`
- US cup: `cup`, `cups`
- US pint: `pint`, `pints`
- US quart: `quart`, `quarts`
- US gallon: `gallon`, `gallons`

Volume conversions use milliliters as the canonical base unit internally.

### Potency

- Percent: `%`, `percent`, `percentage`
- Milligrams per gram: `mg/g`, `mgg`, `mg per g`, `mg/g dry weight`, `milligrams per gram`

Potency conversions use milligrams per gram as the canonical base unit internally.

## Library Usage

```rust
use unitmath::{
    calculate_total_quantity, calculate_total_units, convert_parsed_potency,
    convert_parsed_volume, convert_parsed_weight, convert_potency, convert_volume,
    convert_weight, parse_potency, parse_volume, parse_weight, PotencyUnit, VolumeUnit,
    WeightUnit,
};

let grams = convert_weight(1000.0, WeightUnit::Milligram, WeightUnit::Gram);
assert_eq!(grams, 1.0);

let liters = convert_volume(1000.0, VolumeUnit::Milliliter, VolumeUnit::Liter);
assert_eq!(liters, 1.0);

let milligrams_per_gram =
    convert_potency(22.4, PotencyUnit::Percent, PotencyUnit::MilligramsPerGram);
assert_eq!(milligrams_per_gram, 224.0);

let units = calculate_total_units(2.0, 12.0);
assert_eq!(units, 24.0);

let total_milligrams = calculate_total_quantity(24.0, 100.0);
assert_eq!(total_milligrams, 2400.0);

let parsed_weight = parse_weight("  3.5G  ")?;
assert_eq!(parsed_weight.value, 3.5);
assert_eq!(parsed_weight.unit, WeightUnit::Gram);

let parsed_volume = parse_volume("8 fl oz")?;
assert_eq!(parsed_volume.unit, VolumeUnit::FluidOunce);

let parsed_potency = parse_potency("22.4%")?;
assert_eq!(parsed_potency.unit, PotencyUnit::Percent);

let ounces = convert_parsed_weight("3.5g", WeightUnit::Ounce)?;
let cups = convert_parsed_volume("8 fl oz", VolumeUnit::Cup)?;
let percent = convert_parsed_potency("224mg/g", PotencyUnit::Percent)?;

assert!(ounces > 0.0);
assert_eq!(cups, 1.0);
assert_eq!(percent, 22.4);

# Ok::<(), unitmath::UnitMathError>(())
```

Parsing trims whitespace, is case-insensitive, and returns `UnitMathError` for empty input, missing numbers, invalid numbers, missing units, and unknown units.

## CLI Examples

Default CLI output is numeric-only:

```sh
unitmath weight "1000mg" g
unitmath weight "1 lb" oz
unitmath volume "8 fl oz" cup
unitmath volume "1 gallon" ml
unitmath potency "22.4%" mg/g
unitmath potency "224mg/g" percent
unitmath convert "3.5g" oz
unitmath convert "1 gallon" ml
unitmath convert "22.4%" mg/g
```

Package CLI commands perform simple multiplication without unit conversion:

```sh
unitmath package total-units "2 x 12"
unitmath package total-units "2,12"
unitmath package total-quantity "10 x 3.5"
unitmath package total-quantity "24 * 100" mg
```

Add `--precision <digits>` to format output values with exactly that many digits after the decimal point. Precision supports values from `0` through `12` and affects output formatting only:

```sh
unitmath convert "3.5g" oz --precision 4
unitmath convert "3.5g" oz --json --precision 4
unitmath convert "3.5g" oz --csv --precision 4
unitmath package total-units "2 x 12" --precision 2
```

## Machine-Readable Output

One-shot commands support JSON and CSV-style output:

```sh
unitmath convert "3.5g" oz --json
unitmath convert "3.5g" oz --csv
unitmath convert "3.5g" oz --csv --no-header
unitmath convert "3.5g" oz --csv --include-header
unitmath convert "3.5g" oz --csv --delimiter tab
```

One-shot JSON output is a single object:

```json
{"category":"weight","input":"3.5g","target_unit":"oz","value":0.12345886682353144}
```

One-shot CSV output includes headers by default:

```csv
category,input,target_unit,value
weight,3.5g,oz,0.12345886682353144
```

Batch commands support CSV-style rows, JSON Lines, and JSON arrays:

```sh
unitmath batch examples/conversions.csv --csv
unitmath batch examples/conversions.csv --json
unitmath batch examples/conversions.csv --json --json-array
unitmath batch examples/conversions.csv --csv --delimiter tab
unitmath batch examples/conversions.csv --csv --delimiter pipe
```

Batch JSON output is JSON Lines by default:

```jsonl
{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null}
{"category":"volume","input":"1 gallon","target_unit":"ml","value":3785.411784,"status":"ok","error":null}
```

Add `--json-array` with batch `--json` to emit one JSON array instead of JSON Lines:

```json
[{"category":"weight","input":"1000mg","target_unit":"g","value":1,"status":"ok","error":null},{"category":"volume","input":"1 gallon","target_unit":"ml","value":3785.411784,"status":"ok","error":null}]
```

CSV-style output uses commas by default. Use `--delimiter comma`, `--delimiter tab`, or `--delimiter pipe` with `--csv` to choose the output separator. Delimiter control affects output only; batch CSV input remains comma-separated.

## Batch Examples

Batch CSV input uses these headers:

```csv
category,input,target_unit
weight,1000mg,g
volume,1 gallon,ml
potency,22.4%,mg/g
convert,8 fl oz,cup
total_units,2 x 12,units
total_quantity,24 * 100,mg
```

Supported batch categories are `weight`, `volume`, `potency`, `convert`, `total_units`, and `total_quantity`. Surrounding whitespace in header names and category values is ignored. Package rows do simple multiplication without unit conversion; `target_unit` is preserved as an output label.

JSON Lines batch input uses one flat object per non-empty line with string fields for `category`, `input`, and `target_unit`:

```jsonl
{"category":"weight","input":"1000mg","target_unit":"g"}
{"category":"volume","input":"1 gallon","target_unit":"ml"}
{"category":"potency","input":"22.4%","target_unit":"mg/g"}
{"category":"convert","input":"8 fl oz","target_unit":"cup"}
{"category":"total_units","input":"2 x 12","target_unit":"units"}
{"category":"total_quantity","input":"24 * 100","target_unit":"mg"}
```

Copy-paste batch commands:

```sh
unitmath batch examples/conversions.csv --csv
unitmath batch examples/conversions.csv --json
unitmath batch examples/conversions.csv --json --json-array
unitmath batch examples/conversions.jsonl --json
unitmath batch examples/conversions.jsonl --csv
unitmath batch examples/conversions.csv --csv --precision 2
```

Omit the file path to read from stdin. Stdin defaults to CSV unless `--input-format jsonl` or `--input-json` is provided:

```sh
cat examples/conversions.csv | unitmath batch --csv
cat examples/conversions.csv | unitmath batch --json
cat examples/conversions.jsonl | unitmath batch --input-format jsonl --csv
cat examples/conversions.jsonl | unitmath batch --input-format jsonl --json
```

Batch file input is auto-detected from the file extension:

- `.csv` reads CSV input
- `.jsonl` reads JSON Lines input
- `.ndjson` reads JSON Lines input

Extension matching is case-insensitive. Files with unknown or missing extensions require `--input-format csv` or `--input-format jsonl`:

```sh
unitmath batch examples/conversions.csv --input-format csv --csv
unitmath batch examples/conversions.jsonl --input-format jsonl --json
unitmath batch examples/conversions.jsonl --input-format jsonl --json --json-array
```

The older `--input-json` flag remains available as a compatibility alias for `--input-format jsonl`:

```sh
unitmath batch examples/conversions.jsonl --input-json --json
cat examples/conversions.jsonl | unitmath batch --input-json --json
```

Use `--out <path>` to write batch results to a file instead of stdout:

```sh
unitmath batch examples/conversions.csv --csv --out results.csv
unitmath batch examples/conversions.csv --json --out results.jsonl
unitmath batch examples/conversions.csv --json --json-array --out results.json
unitmath batch examples/conversions.csv --csv --delimiter tab --out results.tsv
cat examples/conversions.csv | unitmath batch --csv --out results.csv
```

## Dirty Data Workflows

Rows with conversion errors are included in batch output with `status` set to `error`; batch processing continues after row-level failures. Use filters and summaries to triage mixed-quality files:

```sh
unitmath batch examples/conversions.csv --csv --errors-only
unitmath batch examples/conversions.csv --json --errors-only
unitmath batch examples/conversions.csv --csv --ok-only
unitmath batch examples/conversions.csv --csv --summary
unitmath batch examples/conversions.csv --csv --errors-only --summary
unitmath batch examples/conversions.csv --csv --errors-only --out errors.csv
unitmath batch examples/conversions.csv --json --ok-only --out clean.jsonl
```

`--summary` writes counts to stderr and never changes stdout or output file schemas:

```text
summary: processed=6 ok=6 errors=0 emitted=6
summary: processed=6 ok=5 errors=1 emitted=1 output=errors.csv
```

Summary counts are:

- `processed`: all parsed batch result rows before filtering
- `ok`: successful rows before filtering
- `errors`: error rows before filtering
- `emitted`: rows emitted after `--errors-only` or `--ok-only`

## Examples

Library examples:

```sh
cargo run --example basic_weight
cargo run --example basic_volume
cargo run --example basic_potency
```

Batch sample files:

- `examples/conversions.csv`
- `examples/conversions.jsonl`

## Roadmap

- Additional parsers
- More unit families
- Richer package parsing as a library API
- Broader CLI ergonomics
- Trait-based quantity abstractions when the core API shape is clear
