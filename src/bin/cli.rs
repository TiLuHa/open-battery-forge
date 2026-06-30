use std::collections::HashMap;

use ebc_hub::config::Config;
use ebc_hub::db_access::Storage;
use ebc_hub::db_access::models::{BatteryIntake, Test, TestConfig};
use ebc_hub::ebc;
use ebc_hub::ebc::frame::OutboundFrame;
use ebc_hub::ebc_runner::{self, EbcRunner};

use color_eyre::eyre::{Result, eyre};
use sqlx::SqlitePool;
use tokio::io::{self, AsyncBufReadExt, BufReader};

#[derive(Debug, Clone, Copy)]
enum CliMode {
    DscCc,
    DscCp,
    ChgCv,
}

impl std::str::FromStr for CliMode {
    type Err = color_eyre::Report;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "DSC-CC" | "dsc-cc" => Ok(Self::DscCc),
            "DSC-CP" | "dsc-cp" => Ok(Self::DscCp),
            "CHG-CV" | "chg-cv" => Ok(Self::ChgCv),
            _ => Err(eyre!("Unknown mode: {s}")),
        }
    }
}

fn print_help() {
    println!("Commands:");
    println!("  connect <id>");
    println!("  status <id>");
    println!("  start <id> <mode> <value1> <value2> <value3>");
    println!("  adjust <id> <mode> <value1> <value2> <value3>");
    println!("  continue <id> <mode> <value1> <value2> <value3>");
    println!("  stop <id>");
    println!("  disconnect <id>");
    println!("  quit");
    println!("  battery-type list");
    println!(
        "  battery-type add <manufacturer> <model> <chemistry> <nominal_voltage_mv> <nominal_capacity_mah> <charge_termination_voltage_mv> <discharge_cutoff_voltage_mv>"
    );
    println!("  battery add <battery_id> <battery_type_id>");
    println!("  battery list");
    println!("  battery show <battery_id>");
    println!("  battery-intake show <battery_id>");
    println!("  battery-intake set <battery_id> <voltage_mv> <resistance_uohm>");
    println!(
        "  test create <battery_id> <device_id> <mode> <voltage_before_test_mv> <value1> <value2> <value3>"
    );
    println!("  test list");
    println!("  test show <test_id>");
    println!("  test approve <test_id>");
    println!("  test delete <test_id>");
    println!("  session create <test_id> [reason]");
    println!("  session end <session_id>");
    println!("  session list <test_id>");
    println!(
        "  sample append <session_id> <elapsed_ms> <voltage_mv> <current_ma> <capacity_mah> [energy_mwh]"
    );
    println!("  sample list <session_id>");
    println!();
    println!("Modes:");
    println!("  DSC-CC - Constant Current Discharge");
    println!("           value1=current_mA value2=cutoff_mV value3=time_min");
    println!("  DSC-CP - Constant Power Discharge");
    println!("           value1=power_W    value2=cutoff_mV value3=time_min");
    println!("  CHG-CV - Constant Current And Voltage Charge");
    println!("           value1=current_mA value2=voltage_mV value3=cutoff_current_mA");
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    dotenvy::dotenv()?;

    let pool = SqlitePool::connect("sqlite:data/ebc-hub.db").await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let text = std::fs::read_to_string("config/config.toml")?;
    let config: Config = toml::from_str(&text)?;

    let storage = Storage::connect("sqlite:data/ebc-hub.db").await?;

    let mut ebc_runners = HashMap::new();

    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    println!("EBC CLI ready.");
    print_help();
    print!("> ");

    while let Some(line) = lines.next_line().await? {
        let parts: Vec<_> = line.split_whitespace().collect();

        match parts.as_slice() {
            ["quit"] | ["exit"] => break,

            ["help"] => {
                print_help();
            }

            ["connect", id] => {
                let id = id.parse::<usize>()?.to_string();

                println!("connect {id}");

                let config = match config.ebc.get(&id) {
                    Some(config) => config,
                    None => {
                        println!("The given id is not present.");
                        continue;
                    }
                };

                let result = (|| -> Result<()> {
                    let mut ebc = ebc::Device::new(&config.port)?;
                    ebc.send(OutboundFrame::connect())?;
                    let ebc_runner = EbcRunner::new(ebc)?;
                    let ebc_runner_cmd_tx = ebc_runner.cmd_tx();

                    let ebc_runner_thread: tokio::task::JoinHandle<()> =
                        tokio::task::spawn(ebc_runner.run());
                    ebc_runners.insert(id, (ebc_runner_cmd_tx, ebc_runner_thread));
                    Ok(())
                })();
                if let Err(rep) = result {
                    println!("Error connecting to the device: {:?}", rep);
                }
            }

            ["disconnect", id] => {
                let id = id.parse::<usize>()?.to_string();

                println!("disconnect {id}");

                let (_, handle) = match ebc_runners.remove(&id) {
                    Some(config) => config,
                    None => {
                        println!("The given id is not present.");
                        continue;
                    }
                };
                handle.abort();
            }

            ["status", id] => {
                let id = id.parse::<usize>()?.to_string();

                let (cmd_tx, _) = match ebc_runners.get(&id) {
                    Some(config) => config,
                    None => {
                        println!("The given id is not present.");
                        continue;
                    }
                };

                match ebc_runner::Command::status(cmd_tx.clone()).await {
                    Ok(report) => println!("{report:#?}"),
                    Err(err) => println!("Error when trying to get status: {err:?}"),
                }
            }

            ["start", id, mode, value1, value2, value3] => {
                let id = id.parse::<usize>()?.to_string();
                let mode = mode.parse::<CliMode>()?;
                let value1 = value1.parse::<u16>()?;
                let value2 = value2.parse::<u16>()?;
                let value3 = value3.parse::<u16>()?;

                println!(
                    "start id={id}, mode={mode:?}, value1={value1}, value2={value2}, value3={value3}"
                );

                let (cmd_tx, _) = match ebc_runners.get(&id) {
                    Some(config) => config,
                    None => {
                        println!("The given id is not present.");
                        continue;
                    }
                };

                let result = match mode {
                    CliMode::DscCc => {
                        ebc_runner::Command::start_constant_current_discharge_command(
                            cmd_tx.clone(),
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::DscCp => {
                        ebc_runner::Command::start_constant_power_discharge_command(
                            cmd_tx.clone(),
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::ChgCv => {
                        ebc_runner::Command::start_constant_current_voltage_charge_command(
                            cmd_tx.clone(),
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                };

                if let Err(err) = result {
                    println!("Error when trying to start: {err:?}");
                }
            }

            ["adjust", id, mode, value1, value2, value3] => {
                let id = id.parse::<usize>()?.to_string();
                let mode = mode.parse::<CliMode>()?;
                let value1 = value1.parse::<u16>()?;
                let value2 = value2.parse::<u16>()?;
                let value3 = value3.parse::<u16>()?;

                println!(
                    "adjust id={id}, mode={mode:?}, value1={value1}, value2={value2}, value3={value3}"
                );

                let (cmd_tx, _) = match ebc_runners.get(&id) {
                    Some(config) => config,
                    None => {
                        println!("The given id is not present.");
                        continue;
                    }
                };

                let result = match mode {
                    CliMode::DscCc => {
                        ebc_runner::Command::adjust_constant_current_discharge_command(
                            cmd_tx.clone(),
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::DscCp => {
                        ebc_runner::Command::adjust_constant_power_discharge_command(
                            cmd_tx.clone(),
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::ChgCv => {
                        ebc_runner::Command::adjust_constant_current_voltage_charge_command(
                            cmd_tx.clone(),
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                };

                if let Err(err) = result {
                    println!("Error when trying to adjust: {err:?}");
                }
            }

            ["continue", id, mode, value1, value2, value3] => {
                let id = id.parse::<usize>()?.to_string();
                let mode = mode.parse::<CliMode>()?;
                let value1 = value1.parse::<u16>()?;
                let value2 = value2.parse::<u16>()?;
                let value3 = value3.parse::<u16>()?;

                println!(
                    "continue id={id}, mode={mode:?}, value1={value1}, value2={value2}, value3={value3}"
                );

                let (cmd_tx, _) = match ebc_runners.get(&id) {
                    Some(config) => config,
                    None => {
                        println!("The given id is not present.");
                        continue;
                    }
                };
                let result = match mode {
                    CliMode::DscCc => {
                        ebc_runner::Command::continue_constant_current_discharge_command(
                            cmd_tx.clone(),
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::DscCp => {
                        ebc_runner::Command::continue_constant_power_discharge_command(
                            cmd_tx.clone(),
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::ChgCv => {
                        ebc_runner::Command::continue_constant_current_voltage_charge_command(
                            cmd_tx.clone(),
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                };

                if let Err(err) = result {
                    println!("Error when trying to continue: {err:?}");
                }
            }

            ["stop", id] => {
                let id = id.parse::<usize>()?.to_string();

                println!("stop {id}");

                let (cmd_tx, _) = match ebc_runners.get(&id) {
                    Some(config) => config,
                    None => {
                        println!("The given id is not present.");
                        continue;
                    }
                };

                if let Err(err) = ebc_runner::Command::stop(cmd_tx.clone()).await {
                    println!("Error when trying to stop: {err:?}");
                }
            }

            ["battery-type", "list"] => {
                let battery_types = storage.list_battery_types().await?;

                if battery_types.is_empty() {
                    println!("No battery types found.");
                } else {
                    for bt in battery_types {
                        println!(
                            "{}: {} {} | {} | {} mV | {} mAh | {} mV | {} mV",
                            bt.id,
                            bt.manufacturer,
                            bt.model,
                            bt.chemistry,
                            bt.nominal_voltage_mv,
                            bt.nominal_capacity_mah,
                            bt.charge_termination_voltage_mv,
                            bt.discharge_cutoff_voltage_mv
                        );
                    }
                }
            }

            [
                "battery-type",
                "add",
                manufacturer,
                model,
                chemistry,
                voltage_mv,
                capacity_mah,
                charge_termination_voltage_mv,
                discharge_cutoff_voltage_mv,
            ] => {
                let voltage_mv = voltage_mv.parse::<i64>()?;
                let capacity_mah = capacity_mah.parse::<i64>()?;
                let charge_termination_voltage_mv = charge_termination_voltage_mv.parse::<i64>()?;
                let discharge_cutoff_voltage_mv = discharge_cutoff_voltage_mv.parse::<i64>()?;

                let id = storage
                    .create_battery_type(
                        manufacturer,
                        model,
                        chemistry,
                        voltage_mv,
                        capacity_mah,
                        charge_termination_voltage_mv,
                        discharge_cutoff_voltage_mv,
                    )
                    .await?;

                println!("Created battery type with id {id}.");
            }

            ["battery", "add", battery_id, battery_type_id] => {
                let battery_type_id = battery_type_id.parse::<i64>()?;

                storage.create_battery(battery_id, battery_type_id).await?;

                println!("Created battery {battery_id}.");
            }

            ["battery", "list"] => {
                let batteries = storage.list_batteries().await?;

                if batteries.is_empty() {
                    println!("No batteries found.");
                } else {
                    for b in batteries {
                        println!("id={} | type_id={}", b.battery_id, b.battery_type_id);
                    }
                }
            }

            ["battery", "show", battery_id] => match storage.get_battery(battery_id).await? {
                Some(battery) => println!("{battery:#?}"),
                None => println!("Battery not found: {battery_id}"),
            },

            ["battery-intake", "show", battery_id] => {
                match storage.get_battery_intake(battery_id).await? {
                    Some(intake) => println!("{intake:#?}"),
                    None => println!("No intake data found for battery {battery_id}."),
                }
            }

            [
                "battery-intake",
                "set",
                battery_id,
                voltage_mv,
                resistance_uohm,
            ] => {
                let voltage_mv = voltage_mv.parse::<i64>()?;
                let resistance_uohm = resistance_uohm.parse::<i64>()?;

                let mut intake = storage
                    .get_battery_intake(battery_id)
                    .await?
                    .unwrap_or_else(|| BatteryIntake {
                        battery_id: battery_id.to_string(),
                        serial_number: None,
                        purchase_date: None,
                        delivery_date: None,
                        voltage_at_delivery_mv: None,
                        internal_resistance_at_delivery_uohm: None,
                        visual_inspection: None,
                        notes: None,
                    });

                intake.voltage_at_delivery_mv = Some(voltage_mv);
                intake.internal_resistance_at_delivery_uohm = Some(resistance_uohm);

                storage.upsert_battery_intake(&intake).await?;

                println!("Updated intake for {battery_id}: {voltage_mv} mV, {resistance_uohm} µΩ.");
            }

            [
                "test",
                "create",
                battery_id,
                device_id,
                mode,
                voltage_before_test_mv,
                value1,
                value2,
                value3,
            ] => {
                let value1 = value1.parse::<i64>()?;
                let value2 = value2.parse::<i64>()?;
                let value3 = value3.parse::<i64>()?;
                let voltage_before_test_mv = voltage_before_test_mv.parse::<i64>()?;

                let config = match *mode {
                    "DSC-CC" => TestConfig::DischargeConstantCurrent {
                        target_current_ma: value1,
                        cutoff_voltage_mv: value2,
                        cutoff_time_min: value3,
                    },
                    "DSC-CP" => TestConfig::DischargeConstantPower {
                        target_power_w: value1,
                        cutoff_voltage_mv: value2,
                        cutoff_time_min: value3,
                    },
                    "CHG-CV" => TestConfig::ChargeConstantVoltage {
                        target_current_ma: value1,
                        charge_voltage_mv: value2,
                        charge_cutoff_current_ma: value3,
                    },
                    _ => {
                        println!("Unknown mode: {mode}");
                        print!("> ");
                        continue;
                    }
                };

                let test = Test {
                    id: 0,
                    battery_id: battery_id.to_string(),
                    approved: false,
                    device_id: device_id.to_string(),
                    mode: mode.to_string().into(),

                    voltage_before_test_mv,
                    config,

                    measured_capacity_mah: None,
                    measured_energy_mwh: None,
                    end_voltage_mv: None,

                    notes: None,
                };

                let id = storage.create_test(&test).await?;

                println!("Created test {id}.");
            }

            ["test", "list"] => {
                let tests = storage.list_tests().await?;

                if tests.is_empty() {
                    println!("No tests found.");
                } else {
                    println!("ID | Battery | Mode | Approved | Device");
                    println!("----------------------------------------");

                    for t in tests {
                        println!(
                            "{} | {} | {} | {} | {}",
                            t.id,
                            t.battery_id,
                            t.mode.acronym,
                            if t.approved { "yes" } else { "no" },
                            t.device_id
                        );
                    }
                }
            }

            ["test", "show", id] => {
                let id = id.parse::<i64>()?;

                match storage.get_test(id).await? {
                    Some(test) => println!("{test:#?}"),
                    None => println!("Test not found: {id}"),
                }
            }

            ["test", "approve", id] => {
                let id = id.parse::<i64>()?;

                storage.approve_test(id).await?;

                println!("Approved test {id}.");
            }

            ["test", "delete", id] => {
                let id = id.parse::<i64>()?;

                storage.delete_test(id).await?;

                println!("Deleted test {id}.");
            }

            ["session", "create", test_id] => {
                let test_id = test_id.parse::<i64>()?;
                let session_id = storage.create_test_session(test_id, None).await?;
                println!("Created session {session_id}.");
            }

            ["session", "create", test_id, reason] => {
                let test_id = test_id.parse::<i64>()?;
                let session_id = storage.create_test_session(test_id, Some(reason)).await?;
                println!("Created session {session_id}.");
            }

            ["session", "end", session_id] => {
                let session_id = session_id.parse::<i64>()?;
                storage.end_test_session(session_id).await?;
                println!("Ended session {session_id}.");
            }

            ["session", "list", test_id] => {
                let test_id = test_id.parse::<i64>()?;
                let sessions = storage.list_test_sessions(test_id).await?;

                if sessions.is_empty() {
                    println!("No sessions found for test {test_id}.");
                } else {
                    for s in sessions {
                        println!("{s:#?}");
                    }
                }
            }

            [
                "sample",
                "append",
                session_id,
                elapsed_ms,
                voltage_mv,
                current_ma,
                capacity_mah,
            ] => {
                let session_id = session_id.parse::<i64>()?;
                let elapsed_ms = elapsed_ms.parse::<i64>()?;
                let voltage_mv = voltage_mv.parse::<i64>()?;
                let current_ma = current_ma.parse::<i64>()?;
                let capacity_mah = capacity_mah.parse::<i64>()?;

                let sample_index = storage
                    .append_sample_auto_index(
                        session_id,
                        elapsed_ms,
                        voltage_mv,
                        current_ma,
                        capacity_mah,
                        None,
                    )
                    .await?;

                println!("Appended sample {sample_index} to session {session_id}.");
            }

            [
                "sample",
                "append",
                session_id,
                elapsed_ms,
                voltage_mv,
                current_ma,
                capacity_mah,
                energy_mwh,
            ] => {
                let session_id = session_id.parse::<i64>()?;
                let elapsed_ms = elapsed_ms.parse::<i64>()?;
                let voltage_mv = voltage_mv.parse::<i64>()?;
                let current_ma = current_ma.parse::<i64>()?;
                let capacity_mah = capacity_mah.parse::<i64>()?;
                let energy_mwh = energy_mwh.parse::<i64>()?;

                let sample_index = storage
                    .append_sample_auto_index(
                        session_id,
                        elapsed_ms,
                        voltage_mv,
                        current_ma,
                        capacity_mah,
                        Some(energy_mwh),
                    )
                    .await?;

                println!("Appended sample {sample_index} to session {session_id}.");
            }

            ["sample", "list", session_id] => {
                let session_id = session_id.parse::<i64>()?;
                let samples = storage.list_samples_for_session(session_id).await?;

                if samples.is_empty() {
                    println!("No samples found for session {session_id}.");
                } else {
                    for sample in samples {
                        println!("{sample:#?}");
                    }
                }
            }

            [] => {}

            cmd => {
                println!("Unknown command: {cmd:?}");
                print_help();
            }
        }

        print!("> ");
    }

    Ok(())
}
