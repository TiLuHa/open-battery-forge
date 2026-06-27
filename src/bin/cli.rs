use std::collections::HashMap;

use ebc_hub::db_access::Storage;
use ebc_hub::db_access::models::{BatteryIntake, Test};
use ebc_hub::ebc_manager::EbcManager;
use ebc_hub::{config::Config, ebc_manager_commands::EbcCommand};

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
    println!("  test create <battery_id> <device_id> <mode> <voltage_before_test_mv> <value1> <value2> <value3>");
    println!("  test list");
    println!("  test show <test_id>");
    println!("  test approve <test_id>");
    println!("  test delete <test_id>");
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

    let ebc_manager = EbcManager::new(config.ebc);
    let ebc_manager_cmd_tx = ebc_manager.cmd_tx();

    let _ebc_manager_thread = tokio::task::spawn(ebc_manager.run());

    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    let mut ebcs = HashMap::new();

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

                match EbcCommand::connect(ebc_manager_cmd_tx.clone(), id.clone()).await {
                    Ok(dev_info) => {
                        ebcs.insert(id, dev_info);
                    }
                    Err(err) => {
                        println!("Error when trying to connect: {err:?}");
                    }
                }
            }

            ["status", id] => {
                let id = id.parse::<usize>()?.to_string();

                match EbcCommand::status(ebc_manager_cmd_tx.clone(), id).await {
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

                let result = match mode {
                    CliMode::DscCc => {
                        EbcCommand::start_constant_current_discharge_command(
                            ebc_manager_cmd_tx.clone(),
                            id,
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::DscCp => {
                        EbcCommand::start_constant_power_discharge_command(
                            ebc_manager_cmd_tx.clone(),
                            id,
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::ChgCv => {
                        EbcCommand::start_constant_current_voltage_charge_command(
                            ebc_manager_cmd_tx.clone(),
                            id,
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

                let result = match mode {
                    CliMode::DscCc => {
                        EbcCommand::adjust_constant_current_discharge_command(
                            ebc_manager_cmd_tx.clone(),
                            id,
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::DscCp => {
                        EbcCommand::adjust_constant_power_discharge_command(
                            ebc_manager_cmd_tx.clone(),
                            id,
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::ChgCv => {
                        EbcCommand::adjust_constant_current_voltage_charge_command(
                            ebc_manager_cmd_tx.clone(),
                            id,
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

                let result = match mode {
                    CliMode::DscCc => {
                        EbcCommand::continue_constant_current_discharge_command(
                            ebc_manager_cmd_tx.clone(),
                            id,
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::DscCp => {
                        EbcCommand::continue_constant_power_discharge_command(
                            ebc_manager_cmd_tx.clone(),
                            id,
                            value1,
                            value2,
                            value3,
                        )
                        .await
                    }
                    CliMode::ChgCv => {
                        EbcCommand::continue_constant_current_voltage_charge_command(
                            ebc_manager_cmd_tx.clone(),
                            id,
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

                if let Err(err) = EbcCommand::stop(ebc_manager_cmd_tx.clone(), id).await {
                    println!("Error when trying to stop: {err:?}");
                }
            }

            ["disconnect", id] => {
                let id = id.parse::<usize>()?.to_string();

                println!("disconnect {id}");

                if let Err(err) = EbcCommand::disconnect(ebc_manager_cmd_tx.clone(), id).await {
                    println!("Error when trying to disconnect: {err:?}");
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

                let mut test = Test {
                    id: 0,
                    battery_id: battery_id.to_string(),
                    approved: false,
                    device_id: device_id.to_string(),
                    mode: mode.to_string(),

                    voltage_before_test_mv,

                    target_current_ma: None,
                    target_power_w: None,
                    cutoff_voltage_mv: None,
                    cutoff_time_min: None,
                    charge_voltage_mv: None,
                    charge_cutoff_current_ma: None,

                    measured_capacity_mah: None,
                    measured_energy_mwh: None,
                    end_voltage_mv: None,

                    notes: None,
                };

                match *mode {
                    "DSC-CC" => {
                        test.target_current_ma = Some(value1);
                        test.cutoff_voltage_mv = Some(value2);
                        test.cutoff_time_min = Some(value3);
                    }
                    "DSC-CP" => {
                        test.target_power_w = Some(value1);
                        test.cutoff_voltage_mv = Some(value2);
                        test.cutoff_time_min = Some(value3);
                    }
                    "CHG-CV" => {
                        test.target_current_ma = Some(value1);
                        test.charge_voltage_mv = Some(value2);
                        test.charge_cutoff_current_ma = Some(value3);
                    }
                    _ => {
                        println!("Unknown mode: {mode}");
                        print!("> ");
                        continue;
                    }
                }

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
                            t.mode,
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
