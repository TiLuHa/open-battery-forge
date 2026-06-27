use color_eyre::eyre::Result;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

use crate::db_access::models::{Battery, BatteryIntake, BatteryType, Test};

#[derive(Clone)]
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&pool)
            .await?;

        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn list_battery_types(&self) -> Result<Vec<BatteryType>> {
        let rows = sqlx::query_as!(
            BatteryType,
            r#"
            SELECT
                id,
                manufacturer,
                model,
                chemistry,
                nominal_voltage_mv,
                nominal_capacity_mah,
                charge_termination_voltage_mv,
                discharge_cutoff_voltage_mv,
                notes
            FROM battery_types
            ORDER BY manufacturer, model
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_battery_type(
        &self,
        manufacturer: &str,
        model: &str,
        chemistry: &str,
        nominal_voltage_mv: i64,
        nominal_capacity_mah: i64,
        charge_termination_voltage_mv: i64,
        discharge_cutoff_voltage_mv: i64,
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO battery_types (
                manufacturer,
                model,
                chemistry,
                nominal_voltage_mv,
                nominal_capacity_mah,
                charge_termination_voltage_mv,
                discharge_cutoff_voltage_mv
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            manufacturer,
            model,
            chemistry,
            nominal_voltage_mv,
            nominal_capacity_mah,
            charge_termination_voltage_mv,
            discharge_cutoff_voltage_mv,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn list_batteries(&self) -> Result<Vec<Battery>> {
        let result = sqlx::query_as!(
            Battery,
            r#"
            SELECT
                battery_id as "battery_id!",
                battery_type_id as "battery_type_id!",
                notes
            FROM batteries
            ORDER BY battery_id
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(result)
    }

    pub async fn get_battery(&self, battery_id: &str) -> Result<Option<Battery>> {
        let result = sqlx::query_as!(
            Battery,
            r#"
            SELECT
                battery_id as "battery_id!",
                battery_type_id as "battery_type_id!",
                notes
            FROM batteries
            WHERE battery_id = ?
            "#,
            battery_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(result)
    }

    pub async fn create_battery(&self, battery_id: &str, battery_type_id: i64) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO batteries (
                battery_id,
                battery_type_id
            )
            VALUES (?, ?)
            "#,
            battery_id,
            battery_type_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_battery_intake(&self, battery_id: &str) -> Result<Option<BatteryIntake>> {
        let result = sqlx::query_as!(
            BatteryIntake,
            r#"
            SELECT
                battery_id as "battery_id!",
                serial_number,
                purchase_date,
                delivery_date,
                voltage_at_delivery_mv,
                internal_resistance_at_delivery_uohm,
                visual_inspection,
                notes
            FROM battery_intake
            WHERE battery_id = ?
            "#,
            battery_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    pub async fn upsert_battery_intake(&self, intake: &BatteryIntake) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO battery_intake (
                battery_id,
                serial_number,
                purchase_date,
                delivery_date,
                voltage_at_delivery_mv,
                internal_resistance_at_delivery_uohm,
                visual_inspection,
                notes
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)

            ON CONFLICT(battery_id)
            DO UPDATE SET
                serial_number = excluded.serial_number,
                purchase_date = excluded.purchase_date,
                delivery_date = excluded.delivery_date,
                voltage_at_delivery_mv = excluded.voltage_at_delivery_mv,
                internal_resistance_at_delivery_uohm = excluded.internal_resistance_at_delivery_uohm,
                visual_inspection = excluded.visual_inspection,
                notes = excluded.notes
            "#,
            intake.battery_id,
            intake.serial_number,
            intake.purchase_date,
            intake.delivery_date,
            intake.voltage_at_delivery_mv,
            intake.internal_resistance_at_delivery_uohm,
            intake.visual_inspection,
            intake.notes,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
    pub async fn create_test(&self, test: &Test) -> Result<i64> {
        let result = sqlx::query!(
            r#"
        INSERT INTO tests (
            battery_id,
            approved,
            device_id,
            mode,
            voltage_before_test_mv,
            target_current_ma,
            target_power_w,
            cutoff_voltage_mv,
            cutoff_time_min,
            charge_voltage_mv,
            charge_cutoff_current_ma,
            measured_capacity_mah,
            measured_energy_mwh,
            end_voltage_mv,
            notes
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
            test.battery_id,
            test.approved,
            test.device_id,
            test.mode,
            test.voltage_before_test_mv,
            test.target_current_ma,
            test.target_power_w,
            test.cutoff_voltage_mv,
            test.cutoff_time_min,
            test.charge_voltage_mv,
            test.charge_cutoff_current_ma,
            test.measured_capacity_mah,
            test.measured_energy_mwh,
            test.end_voltage_mv,
            test.notes,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn list_tests(&self) -> Result<Vec<Test>> {
        let tests = sqlx::query_as!(
            Test,
            r#"
        SELECT
            id as "id!",
            battery_id as "battery_id!",
            approved as "approved!: bool",
            device_id as "device_id!",
            mode as "mode!",
            voltage_before_test_mv,
            target_current_ma,
            target_power_w,
            cutoff_voltage_mv,
            cutoff_time_min,
            charge_voltage_mv,
            charge_cutoff_current_ma,
            measured_capacity_mah,
            measured_energy_mwh,
            end_voltage_mv,
            notes
        FROM tests
        ORDER BY id DESC
        "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(tests)
    }

    pub async fn get_test(&self, id: i64) -> Result<Option<Test>> {
        let test = sqlx::query_as!(
            Test,
            r#"
        SELECT
            id as "id!",
            battery_id as "battery_id!",
            approved as "approved!: bool",
            device_id as "device_id!",
            mode as "mode!",
            voltage_before_test_mv,
            target_current_ma,
            target_power_w,
            cutoff_voltage_mv,
            cutoff_time_min,
            charge_voltage_mv,
            charge_cutoff_current_ma,
            measured_capacity_mah,
            measured_energy_mwh,
            end_voltage_mv,
            notes
        FROM tests
        WHERE id = ?
        "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(test)
    }

    pub async fn approve_test(&self, id: i64) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let test = sqlx::query!(
            r#"
        SELECT battery_id as "battery_id!"
        FROM tests
        WHERE id = ?
        "#,
            id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| color_eyre::eyre::eyre!("Test not found: {id}"))?;

        sqlx::query!(
            r#"
        UPDATE tests
        SET approved = 0
        WHERE battery_id = ?
        "#,
            test.battery_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
        UPDATE tests
        SET approved = 1
        WHERE id = ?
        "#,
            id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn delete_test(&self, id: i64) -> Result<()> {
        sqlx::query!(
            r#"
        DELETE FROM tests
        WHERE id = ?
        "#,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
