use crate::db_access::models::TestMode;

#[derive(Debug, Clone)]
pub enum TestConfig {
    DischargeConstantCurrent {
        target_current_ma: i64,
        cutoff_voltage_mv: i64,
        cutoff_time_min: i64,
    },
    DischargeConstantPower {
        target_power_w: i64,
        cutoff_voltage_mv: i64,
        cutoff_time_min: i64,
    },
    ChargeConstantVoltage {
        target_current_ma: i64,
        charge_voltage_mv: i64,
        charge_cutoff_current_ma: i64,
    },
}

#[derive(Debug, Clone)]
pub struct Test {
    pub id: i64,
    pub battery_id: String,
    pub approved: bool,
    pub device_id: String,
    pub mode: TestMode,
    pub voltage_before_test_mv: i64,
    pub config: TestConfig,
    pub measured_capacity_mah: Option<i64>,
    pub measured_energy_mwh: Option<i64>,
    pub end_voltage_mv: Option<i64>,
    pub notes: Option<String>,
}
