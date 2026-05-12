use unitmath::{convert_parsed_weight, convert_weight, parse_weight, WeightUnit};

fn main() -> Result<(), unitmath::UnitMathError> {
    let grams = convert_weight(1000.0, WeightUnit::Milligram, WeightUnit::Gram);
    println!("1000 mg = {grams} g");

    let parsed = parse_weight("3.5 g")?;
    println!("parsed weight: {} {:?}", parsed.value, parsed.unit);

    let ounces = convert_parsed_weight("3.5g", WeightUnit::Ounce)?;
    println!("3.5 g = {ounces} oz");

    Ok(())
}
