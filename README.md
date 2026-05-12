# UnitMath

UnitMath is a small Rust library and dependency-free CLI for unit conversions, parsing, and package math.

Version 1.0.0 is a stable MVP focused on:

- Weight conversions
- US liquid volume conversions
- Potency conversions
- Simple package math helpers
- Basic string parsing for weight, volume, and potency quantities
- Parsed conversion helpers
- A minimal CLI

UnitMath intentionally does not include universal conversion, package parsing, JSON output, or trait-based quantity abstractions yet.

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

let total_grams = calculate_total_quantity(10.0, 3.5);
assert_eq!(total_grams, 35.0);

let parsed_weight = parse_weight("  3.5G  ")?;
assert_eq!(parsed_weight.value, 3.5);
assert_eq!(parsed_weight.unit, WeightUnit::Gram);

let parsed_volume = parse_volume("8 fl oz")?;
assert_eq!(parsed_volume.unit, VolumeUnit::FluidOunce);

let parsed_potency = parse_potency("22.4%")?;
assert_eq!(parsed_potency.unit, PotencyUnit::Percent);

let grams = convert_parsed_weight("1000mg", WeightUnit::Gram)?;
assert_eq!(grams, 1.0);

let cups = convert_parsed_volume("8 fl oz", VolumeUnit::Cup)?;
assert_eq!(cups, 1.0);

let milligrams_per_gram =
    convert_parsed_potency("22.4%", PotencyUnit::MilligramsPerGram)?;
assert_eq!(milligrams_per_gram, 224.0);

# Ok::<(), unitmath::UnitMathError>(())
```

## Parsing Examples

```rust
use unitmath::{parse_potency, parse_volume, parse_weight, PotencyUnit, VolumeUnit, WeightUnit};

assert_eq!(parse_weight("1000 mg")?.unit, WeightUnit::Milligram);
assert_eq!(parse_weight("1000mg")?.unit, WeightUnit::Milligram);
assert_eq!(parse_weight(".5 g")?.value, 0.5);

assert_eq!(parse_volume("1000 ml")?.unit, VolumeUnit::Milliliter);
assert_eq!(parse_volume("8floz")?.unit, VolumeUnit::FluidOunce);
assert_eq!(parse_volume("2 gallons")?.unit, VolumeUnit::Gallon);

assert_eq!(parse_potency("22.4 %")?.unit, PotencyUnit::Percent);
assert_eq!(parse_potency("224mg/g")?.unit, PotencyUnit::MilligramsPerGram);
assert_eq!(
    parse_potency("224 milligrams per gram")?.unit,
    PotencyUnit::MilligramsPerGram
);

# Ok::<(), unitmath::UnitMathError>(())
```

Parsing trims whitespace, is case-insensitive, and returns `UnitMathError` for empty input, missing numbers, invalid numbers, missing units, and unknown units.

## Package Helpers

```rust
use unitmath::{calculate_total_quantity, calculate_total_units};

let units = calculate_total_units(5.0, 24.0);
assert_eq!(units, 120.0);

let total_milligrams = calculate_total_quantity(24.0, 100.0);
assert_eq!(total_milligrams, 2400.0);
```

Package helpers only multiply counts by per-unit quantities. They do not parse package strings or perform unit conversion.

## CLI Usage

```sh
unitmath weight "1000mg" g
unitmath weight "1 lb" oz
unitmath volume "8 fl oz" cup
unitmath volume "1 gallon" ml
unitmath potency "22.4%" mg/g
unitmath potency "224mg/g" percent
```

Each command prints only the numeric converted value on success. Errors are written to stderr with usage guidance.

## Examples

```sh
cargo run --example basic_weight
cargo run --example basic_volume
cargo run --example basic_potency
```

## Roadmap After v1.0.0

- Additional parsers
- Package parsing
- More unit families
- Optional structured output modes
- Broader CLI ergonomics
