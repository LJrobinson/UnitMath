/// Units supported for US liquid volume conversion in UnitMath v0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeUnit {
    Milliliter,
    Liter,
    FluidOunce,
    Cup,
    Pint,
    Quart,
    Gallon,
}

impl VolumeUnit {
    const fn milliliters_per_unit(self) -> f64 {
        match self {
            Self::Milliliter => 1.0,
            Self::Liter => 1000.0,
            Self::FluidOunce => 29.573_529_562_5,
            Self::Cup => 236.588_236_5,
            Self::Pint => 473.176_473,
            Self::Quart => 946.352_946,
            Self::Gallon => 3785.411_784,
        }
    }
}

/// Converts a volume value from one unit to another.
///
/// Conversions pass through milliliters as the canonical base unit.
pub fn convert_volume(value: f64, from: VolumeUnit, to: VolumeUnit) -> f64 {
    let milliliters = value * from.milliliters_per_unit();
    milliliters / to.milliliters_per_unit()
}

#[cfg(test)]
mod tests {
    use super::{convert_volume, VolumeUnit};

    fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
        let difference = (actual - expected).abs();
        assert!(
            difference <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}; difference was {difference}"
        );
    }

    #[test]
    fn converts_milliliters_to_liters() {
        let actual = convert_volume(1000.0, VolumeUnit::Milliliter, VolumeUnit::Liter);

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_liters_to_milliliters() {
        let actual = convert_volume(1.0, VolumeUnit::Liter, VolumeUnit::Milliliter);

        assert_approx_eq(actual, 1000.0, 1e-12);
    }

    #[test]
    fn converts_gallons_to_milliliters() {
        let actual = convert_volume(1.0, VolumeUnit::Gallon, VolumeUnit::Milliliter);

        assert_approx_eq(actual, 3785.411_784, 1e-12);
    }

    #[test]
    fn converts_quarts_to_gallons() {
        let actual = convert_volume(4.0, VolumeUnit::Quart, VolumeUnit::Gallon);

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_pints_to_quarts() {
        let actual = convert_volume(2.0, VolumeUnit::Pint, VolumeUnit::Quart);

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_fluid_ounces_to_cups() {
        let actual = convert_volume(8.0, VolumeUnit::FluidOunce, VolumeUnit::Cup);

        assert_approx_eq(actual, 1.0, 1e-12);
    }
}
