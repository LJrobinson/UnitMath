use unitmath::{convert_parsed_potency, convert_potency, parse_potency, PotencyUnit};

fn main() -> Result<(), unitmath::UnitMathError> {
    let milligrams_per_gram =
        convert_potency(22.4, PotencyUnit::Percent, PotencyUnit::MilligramsPerGram);
    println!("22.4% = {milligrams_per_gram} mg/g");

    let parsed = parse_potency("224 mg/g")?;
    println!("parsed potency: {} {:?}", parsed.value, parsed.unit);

    let percent = convert_parsed_potency("224mg/g", PotencyUnit::Percent)?;
    println!("224 mg/g = {percent}%");

    Ok(())
}
