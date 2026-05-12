use unitmath::{convert_parsed_volume, convert_volume, parse_volume, VolumeUnit};

fn main() -> Result<(), unitmath::UnitMathError> {
    let liters = convert_volume(1000.0, VolumeUnit::Milliliter, VolumeUnit::Liter);
    println!("1000 ml = {liters} l");

    let parsed = parse_volume("8 fl oz")?;
    println!("parsed volume: {} {:?}", parsed.value, parsed.unit);

    let cups = convert_parsed_volume("8 fl oz", VolumeUnit::Cup)?;
    println!("8 fl oz = {cups} cup");

    Ok(())
}
