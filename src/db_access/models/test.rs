#[derive(Debug, Clone)]
pub struct Test {
    pub id: i64,
    pub battery_id: String,
    pub approved: bool,
    pub device_id: String,
    pub mode: String,
    pub voltage_before_test_mv: i64,
    pub target_current_ma: Option<i64>,
    pub target_power_w: Option<i64>,
    pub cutoff_voltage_mv: Option<i64>,
    pub cutoff_time_min: Option<i64>,
    pub charge_voltage_mv: Option<i64>,
    pub charge_cutoff_current_ma: Option<i64>,
    pub measured_capacity_mah: Option<i64>,
    pub measured_energy_mwh: Option<i64>,
    pub end_voltage_mv: Option<i64>,
    pub notes: Option<String>,
}
