/// Units supported for weight conversion in UnitMath v0.1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightUnit {
    Milligram,
    Gram,
    Kilogram,
    Ounce,
    Pound,
}

impl WeightUnit {
    const fn grams_per_unit(self) -> f64 {
        match self {
            Self::Milligram => 0.001,
            Self::Gram => 1.0,
            Self::Kilogram => 1000.0,
            Self::Ounce => 28.349_523_125,
            Self::Pound => 453.592_37,
        }
    }
}

/// Converts a weight value from one unit to another.
///
/// Conversions pass through grams as the canonical base unit.
pub fn convert_weight(value: f64, from: WeightUnit, to: WeightUnit) -> f64 {
    let grams = value * from.grams_per_unit();
    grams / to.grams_per_unit()
}

#[cfg(test)]
mod tests {
    use super::{convert_weight, WeightUnit};

    fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
        let difference = (actual - expected).abs();
        assert!(
            difference <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}; difference was {difference}"
        );
    }

    #[test]
    fn converts_milligrams_to_grams() {
        let actual = convert_weight(1000.0, WeightUnit::Milligram, WeightUnit::Gram);

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_kilograms_to_grams() {
        let actual = convert_weight(1.0, WeightUnit::Kilogram, WeightUnit::Gram);

        assert_approx_eq(actual, 1000.0, 1e-12);
    }

    #[test]
    fn converts_ounces_to_grams() {
        let actual = convert_weight(1.0, WeightUnit::Ounce, WeightUnit::Gram);

        assert_approx_eq(actual, 28.349_523_125, 1e-12);
    }

    #[test]
    fn converts_sixteen_ounces_to_one_pound() {
        let actual = convert_weight(16.0, WeightUnit::Ounce, WeightUnit::Pound);

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_one_pound_to_sixteen_ounces() {
        let actual = convert_weight(1.0, WeightUnit::Pound, WeightUnit::Ounce);

        assert_approx_eq(actual, 16.0, 1e-12);
    }

    #[test]
    fn converts_grams_to_ounces() {
        let actual = convert_weight(3.5, WeightUnit::Gram, WeightUnit::Ounce);

        assert_approx_eq(actual, 0.123_459, 1e-6);
    }
}
