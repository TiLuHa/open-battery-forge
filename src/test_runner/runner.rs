use std::time::Duration;

use tokio::sync::{broadcast, mpsc};

use color_eyre::Result;
use tokio::time::Instant;

use crate::db_access::Storage;
use crate::ebc_runner;
use crate::test_runner::{Command, State};

pub struct RunningTest {
    pub test_id: i64,
    pub session_id: i64,
    pub ebc_id: String,
    pub started_at: String,
    pub ebc_event_rx: broadcast::Receiver<ebc_runner::Event>,
}

pub struct TestRunner {
    storage: Storage,
    ebc_cmd_tx: mpsc::UnboundedSender<ebc_runner::Command>,
    cmd_tx: mpsc::UnboundedSender<Command>,
    cmd_rx: mpsc::UnboundedReceiver<Command>,
    reports_rx: broadcast::Receiver<ebc_runner::Event>,
    state: State,
}

impl TestRunner {
    pub async fn new(
        storage: Storage,
        ebc_cmd_tx: mpsc::UnboundedSender<ebc_runner::Command>,
    ) -> Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
        let reports_rx = ebc_runner::Command::sub_reports(ebc_cmd_tx.clone()).await?;
        Ok(TestRunner {
            cmd_rx,
            cmd_tx,
            storage,
            ebc_cmd_tx,
            reports_rx,
            state: State::Idle,
        })
    }

    pub fn cmd_tx(&self) -> mpsc::UnboundedSender<Command> {
        self.cmd_tx.clone()
    }

    async fn handle_start_test(&mut self, test_id: i64) -> Result<()> {
        match &self.state {
            State::Idle | State::Finished => {}
            _ => {
                return Err(color_eyre::eyre::eyre!(
                    "Couldn't resume test, since state is wrong: {:?}",
                    self.state
                ));
            }
        };

        let test = self
            .storage
            .get_test(test_id)
            .await?
            .ok_or_else(|| color_eyre::eyre::eyre!("Test not found: {0}", test_id))?;

        let session_id = self
            .storage
            .create_test_session(test_id, Some("started"))
            .await?;

        match test.config {
            crate::db_access::models::TestParameters::DischargeConstantCurrent {
                target_current_ma,
                cutoff_voltage_mv,
                cutoff_time_min,
            } => {
                ebc_runner::Command::start_constant_current_discharge_command(
                    self.ebc_cmd_tx.clone(),
                    target_current_ma.try_into()?,
                    cutoff_voltage_mv.try_into()?,
                    cutoff_time_min.try_into()?,
                )
                .await?;
            }
            crate::db_access::models::TestParameters::DischargeConstantPower {
                target_power_w,
                cutoff_voltage_mv,
                cutoff_time_min,
            } => {
                ebc_runner::Command::start_constant_power_discharge_command(
                    self.ebc_cmd_tx.clone(),
                    target_power_w.try_into()?,
                    cutoff_voltage_mv.try_into()?,
                    cutoff_time_min.try_into()?,
                )
                .await?;
            }
            crate::db_access::models::TestParameters::ChargeConstantVoltage {
                target_current_ma,
                charge_voltage_mv,
                charge_cutoff_current_ma,
            } => {
                ebc_runner::Command::start_constant_current_voltage_charge_command(
                    self.ebc_cmd_tx.clone(),
                    target_current_ma.try_into()?,
                    charge_voltage_mv.try_into()?,
                    charge_cutoff_current_ma.try_into()?,
                )
                .await?;
            }
        }

        self.state = State::Running { test, session_id };

        Ok(())
    }

    async fn handle_resume_test(&mut self) -> Result<()> {
        let (test, session_id) = match &self.state {
            State::Stopped { test, session_id } => (test, session_id),
            _ => {
                return Err(color_eyre::eyre::eyre!(
                    "Couldn't resume test, since state is wrong: {:?}",
                    self.state
                ));
            }
        };

        match test.config {
            crate::db_access::models::TestParameters::DischargeConstantCurrent {
                target_current_ma,
                cutoff_voltage_mv,
                cutoff_time_min,
            } => {
                ebc_runner::Command::continue_constant_current_discharge_command(
                    self.ebc_cmd_tx.clone(),
                    target_current_ma.try_into()?,
                    cutoff_voltage_mv.try_into()?,
                    cutoff_time_min.try_into()?,
                )
                .await?;
            }
            crate::db_access::models::TestParameters::DischargeConstantPower {
                target_power_w,
                cutoff_voltage_mv,
                cutoff_time_min,
            } => {
                ebc_runner::Command::continue_constant_power_discharge_command(
                    self.ebc_cmd_tx.clone(),
                    target_power_w.try_into()?,
                    cutoff_voltage_mv.try_into()?,
                    cutoff_time_min.try_into()?,
                )
                .await?;
            }
            crate::db_access::models::TestParameters::ChargeConstantVoltage {
                target_current_ma,
                charge_voltage_mv,
                charge_cutoff_current_ma,
            } => {
                ebc_runner::Command::continue_constant_current_voltage_charge_command(
                    self.ebc_cmd_tx.clone(),
                    target_current_ma.try_into()?,
                    charge_voltage_mv.try_into()?,
                    charge_cutoff_current_ma.try_into()?,
                )
                .await?;
            }
        }

        self.state = State::Running {
            test: test.clone(),
            session_id: *session_id,
        };

        Ok(())
    }

    async fn handle_stop_test(&mut self) -> Result<()> {
        let (test, session_id) = match &self.state {
            State::Running { test, session_id } => (test, session_id),
            _ => {
                return Err(color_eyre::eyre::eyre!(
                    "Couldn't resume test, since state is wrong: {:?}",
                    self.state
                ));
            }
        };
        self.state = State::Stopped {
            test: test.clone(),
            session_id: *session_id,
        };
        Ok(())
    }

    async fn handle_cmd(&mut self, cmd: Command) {
        match cmd {
            Command::StartTest { test_id, callback } => {
                callback.send(self.handle_start_test(test_id).await).ok();
            }
            Command::StopTest { callback } => {
                callback.send(self.handle_stop_test().await).ok();
            }
            Command::ResumeTest { callback } => {
                callback.send(self.handle_resume_test().await).ok();
            }
            Command::Status { callback } => {}
        }
    }

    async fn handle_event(&mut self, event: ebc_runner::Event) {}

    pub async fn run(mut self) {
        let mut watchdog = tokio::time::interval(Duration::from_secs(5));
        let mut last_report_at = Instant::now();
        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => {
                    self.handle_cmd(cmd).await;
                }
                Ok(event) = self.reports_rx.recv() => {
                    self.handle_event(event).await;
                }
                // _ = watchdog.tick() => {
                //     if last_report_at.elapsed() > Duration::from_secs(15) {
                //         tracing::warn!(
                //             "test {test_id}: no EBC reports received for {:?}",
                //             last_report_at.elapsed()
                //         );

                //         state = RunningTestState::Failed;
                //         break;
                //     }
                // }
            }
        }
    }
}
