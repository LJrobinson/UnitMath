# UnitMath

UnitMath is a small Rust library for pure unit conversion utilities.

Version 0.8.0 supports weight, US liquid volume, potency conversions, package math helpers, basic string parsing, and parsed conversion helpers.

## Supported Units

### Weight

- Milligram
- Gram
- Kilogram
- Ounce
- Pound

Weight conversions use grams as the canonical base unit internally.

### Volume

- Milliliter
- Liter
- FluidOunce
- Cup
- Pint
- Quart
- Gallon

Volume conversions use milliliters as the canonical base unit internally.

### Potency

- Percent
- MilligramsPerGram

Potency conversions use milligrams per gram as the canonical base unit internally.

### Package Math

- `calculate_total_units(container_count, units_per_container)`
- `calculate_total_quantity(unit_count, quantity_per_unit)`

Package math helpers multiply counts by per-unit quantities without applying unit conversion.

### Weight Parsing

- `parse_weight("1000 mg")`
- `parse_weight("1000mg")`
- `parse_weight("3.5 g")`
- `parse_weight("2 pounds")`

Weight parsing is case-insensitive and returns a `ParsedQuantity<WeightUnit>`.

### Volume Parsing

- `parse_volume("1000 ml")`
- `parse_volume("1000ml")`
- `parse_volume("8 fl oz")`
- `parse_volume("2 gallons")`

Volume parsing is case-insensitive and returns a `ParsedQuantity<VolumeUnit>`.

### Potency Parsing

- `parse_potency("22.4%")`
- `parse_potency("22.4 percent")`
- `parse_potency("224 mg/g")`
- `parse_potency("224 milligrams per gram")`

Potency parsing is case-insensitive and returns a `ParsedQuantity<PotencyUnit>`.

### Parsed Conversion Helpers

- `convert_parsed_weight("1000mg", WeightUnit::Gram)`
- `convert_parsed_volume("8 fl oz", VolumeUnit::Cup)`
- `convert_parsed_potency("22.4%", PotencyUnit::MilligramsPerGram)`

Parsed conversion helpers parse a quantity string and convert it to a target unit in one step.

## Usage

```rust
use unitmath::{
    calculate_total_quantity, calculate_total_units, convert_potency, convert_volume,
    convert_parsed_potency, convert_parsed_volume, convert_parsed_weight, convert_weight,
    parse_potency, parse_volume, parse_weight, PotencyUnit, VolumeUnit, WeightUnit,
};

let grams = convert_weight(1000.0, WeightUnit::Milligram, WeightUnit::Gram);
assert_eq!(grams, 1.0);

let pounds = convert_weight(16.0, WeightUnit::Ounce, WeightUnit::Pound);
assert!((pounds - 1.0).abs() < 1e-12);

let liters = convert_volume(1000.0, VolumeUnit::Milliliter, VolumeUnit::Liter);
assert!((liters - 1.0).abs() < 1e-12);

let milligrams_per_gram = convert_potency(22.4, PotencyUnit::Percent, PotencyUnit::MilligramsPerGram);
assert!((milligrams_per_gram - 224.0).abs() < 1e-12);

let units = calculate_total_units(2.0, 12.0);
assert_eq!(units, 24.0);

let grams = calculate_total_quantity(10.0, 3.5);
assert_eq!(grams, 35.0);

let parsed = parse_weight("3.5 g").unwrap();
assert_eq!(parsed.value, 3.5);
assert_eq!(parsed.unit, WeightUnit::Gram);

let parsed = parse_volume("8 fl oz").unwrap();
assert_eq!(parsed.value, 8.0);
assert_eq!(parsed.unit, VolumeUnit::FluidOunce);

let parsed = parse_potency("22.4%").unwrap();
assert_eq!(parsed.value, 22.4);
assert_eq!(parsed.unit, PotencyUnit::Percent);

let grams = convert_parsed_weight("1000mg", WeightUnit::Gram).unwrap();
assert_eq!(grams, 1.0);

let cups = convert_parsed_volume("8 fl oz", VolumeUnit::Cup).unwrap();
assert!((cups - 1.0).abs() < 1e-12);

let milligrams_per_gram =
    convert_parsed_potency("22.4%", PotencyUnit::MilligramsPerGram).unwrap();
assert_eq!(milligrams_per_gram, 224.0);
```

## Roadmap

- Additional parsers
- CLI
