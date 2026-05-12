/// Units supported for potency conversion in UnitMath v0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotencyUnit {
    Percent,
    MilligramsPerGram,
}

impl PotencyUnit {
    const fn milligrams_per_gram_factor(self) -> f64 {
        match self {
            Self::Percent => 10.0,
            Self::MilligramsPerGram => 1.0,
        }
    }
}

/// Converts a potency value from one unit to another.
///
/// Conversions pass through milligrams per gram as the canonical base unit.
pub fn convert_potency(value: f64, from: PotencyUnit, to: PotencyUnit) -> f64 {
    let milligrams_per_gram = value * from.milligrams_per_gram_factor();
    milligrams_per_gram / to.milligrams_per_gram_factor()
}

#[cfg(test)]
mod tests {
    use super::{convert_potency, PotencyUnit};

    fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
        let difference = (actual - expected).abs();
        assert!(
            difference <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}; difference was {difference}"
        );
    }

    #[test]
    fn converts_one_percent_to_milligrams_per_gram() {
        let actual = convert_potency(1.0, PotencyUnit::Percent, PotencyUnit::MilligramsPerGram);

        assert_approx_eq(actual, 10.0, 1e-12);
    }

    #[test]
    fn converts_twenty_two_point_four_percent_to_milligrams_per_gram() {
        let actual = convert_potency(22.4, PotencyUnit::Percent, PotencyUnit::MilligramsPerGram);

        assert_approx_eq(actual, 224.0, 1e-12);
    }

    #[test]
    fn converts_one_hundred_percent_to_milligrams_per_gram() {
        let actual = convert_potency(100.0, PotencyUnit::Percent, PotencyUnit::MilligramsPerGram);

        assert_approx_eq(actual, 1000.0, 1e-12);
    }

    #[test]
    fn converts_ten_milligrams_per_gram_to_percent() {
        let actual = convert_potency(10.0, PotencyUnit::MilligramsPerGram, PotencyUnit::Percent);

        assert_approx_eq(actual, 1.0, 1e-12);
    }

    #[test]
    fn converts_two_hundred_twenty_four_milligrams_per_gram_to_percent() {
        let actual = convert_potency(224.0, PotencyUnit::MilligramsPerGram, PotencyUnit::Percent);

        assert_approx_eq(actual, 22.4, 1e-12);
    }

    #[test]
    fn converts_one_thousand_milligrams_per_gram_to_percent() {
        let actual = convert_potency(1000.0, PotencyUnit::MilligramsPerGram, PotencyUnit::Percent);

        assert_approx_eq(actual, 100.0, 1e-12);
    }
}
