/// Calculates a total unit count from a number of containers and units per container.
pub fn calculate_total_units(container_count: f64, units_per_container: f64) -> f64 {
    container_count * units_per_container
}

/// Calculates a total quantity from a unit count and quantity per unit.
pub fn calculate_total_quantity(unit_count: f64, quantity_per_unit: f64) -> f64 {
    unit_count * quantity_per_unit
}

#[cfg(test)]
mod tests {
    use super::{calculate_total_quantity, calculate_total_units};

    fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
        let difference = (actual - expected).abs();
        assert!(
            difference <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}; difference was {difference}"
        );
    }

    #[test]
    fn calculates_two_cases_with_twelve_units_each() {
        let actual = calculate_total_units(2.0, 12.0);

        assert_approx_eq(actual, 24.0, 1e-12);
    }

    #[test]
    fn calculates_five_cases_with_twenty_four_units_each() {
        let actual = calculate_total_units(5.0, 24.0);

        assert_approx_eq(actual, 120.0, 1e-12);
    }

    #[test]
    fn calculates_ten_packages_with_three_point_five_grams_each() {
        let actual = calculate_total_quantity(10.0, 3.5);

        assert_approx_eq(actual, 35.0, 1e-12);
    }

    #[test]
    fn calculates_twenty_four_units_with_one_hundred_milligrams_each() {
        let actual = calculate_total_quantity(24.0, 100.0);

        assert_approx_eq(actual, 2400.0, 1e-12);
    }

    #[test]
    fn calculates_zero_containers() {
        let actual = calculate_total_units(0.0, 12.0);

        assert_approx_eq(actual, 0.0, 1e-12);
    }

    #[test]
    fn calculates_fractional_containers() {
        let actual = calculate_total_units(1.5, 10.0);

        assert_approx_eq(actual, 15.0, 1e-12);
    }
}
