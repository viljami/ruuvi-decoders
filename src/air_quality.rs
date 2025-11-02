const AQI_MAX: f64 = 100.0;
const PM25_MAX: f64 = 60.0;
const PM25_MIN: f64 = 0.0;
const PM25_SCALE: f64 = AQI_MAX / (PM25_MAX - PM25_MIN); // ≈ 1.6667
const CO2_MAX: f64 = 2300.0;
const CO2_MIN: f64 = 420.0;
const CO2_SCALE: f64 = AQI_MAX / (CO2_MAX - CO2_MIN); // ≈ 0.05319

#[must_use]
pub fn calc_aqi(pm2_5: f64, co2: u16) -> f64 {
    let pm2_5 = pm2_5.clamp(PM25_MIN, PM25_MAX);
    let co2 = f64::from(co2).clamp(CO2_MIN, CO2_MAX);

    let dx = (pm2_5 - PM25_MIN) * PM25_SCALE; // 0..100
    let dy = (co2 - CO2_MIN) * CO2_SCALE; // 0..100
    let r = f64::hypot(dx, dy); // sqrt(dx*dx + dy*dy)

    (AQI_MAX - r).clamp(0.0, AQI_MAX)
}

#[cfg(test)]
mod tests {
    use std::u16;

    use super::*;
    use rstest::rstest;

    const EPS: f64 = 1e-9;

    // Parameterized tests covering boundaries, midpoints, clamping and expected behavior.
    // Values and expectations are computed using the same formulas as in the implementation:
    // - PM2.5 range 0..60 maps to dx 0..100 via PM25_SCALE = 100/60
    // - CO2 range 420..2300 maps to dy 0..100 via CO2_SCALE = 100/(2300-420)
    // - r = hypot(dx, dy)
    // - aqi = clamp(100 - r, 0, 100)
    #[rstest]
    #[case(0.0, 420, 100.0)] // both minimum -> best AQI
    #[case(60.0, 2300, 0.0)] // both maximum -> worst AQI (clamped)
    #[case(30.0, 1360, 29.28932188134524)] // midpoints -> symmetric -> ~29.2893
    #[case(-10.0, 500, 95.74468085106383)] // pm2_5 clamped to 0
    #[case(10.0, 10000, 0.0)] // co2 clamped to max -> typically worst AQI
    #[case(60.0, 420, 0.0)] // high PM2.5, low CO2 -> dx=100, dy=0 -> r=100 -> AQI 0
    #[case(0.0, 2300, 0.0)] // low PM2.5, high CO2 -> dx=0, dy=100 -> r=100 -> AQI 0
    fn param_calc_aqi(#[case] pm2_5: f64, #[case] co2: u16, #[case] expected: f64) {
        let got = calc_aqi(pm2_5, co2);
        assert!(
            (got - expected).abs() < EPS,
            "calc_aqi({}, {}) = {}, expected {}",
            pm2_5,
            co2,
            got,
            expected
        );
    }

    #[test]
    fn smoke_properties() {
        // Explicit clamping assertions
        // pm2_5 below min should behave like pm2_5 = PM25_MIN
        assert!(
            (calc_aqi(-100.0, 420) - 100.0).abs() < EPS,
            "pm2_5 below minimum should be clamped to PM25_MIN"
        );
        // co2 above max should be clamped to CO2_MAX resulting in r >= 100 -> AQI 0
        assert!(
            (calc_aqi(0.0, u16::MAX) - 0.0).abs() < EPS,
            "co2 above maximum should be clamped to CO2_MAX producing AQI 0"
        );
    }
}
