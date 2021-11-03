#![allow(dead_code)]
/// Specialized node which is supposed to be compatible with 5 last jormungandr releases
use crate::{
    legacy::LegacySettings,
    node::{Error, ProgressBarController, Result},
    style,
};
use chain_impl_mockchain::header::HeaderId;
use jormungandr_lib::multiaddr;
pub use jormungandr_testing_utils::testing::{
    jormungandr::{JormungandrProcess, StartupVerificationMode, Status},
    network::{LeadershipMode, NodeAlias, NodeBlock0, NodeSetting, PersistenceMode, Settings},
    node::{grpc::JormungandrClient, BackwardCompatibleRest, JormungandrLogger, JormungandrRest},
    FragmentNode, FragmentNodeError, MemPoolCheck,
};
use std::io::{BufRead, BufReader};
use std::process::ExitStatus;
use std::time::Duration;
use yaml_rust::{Yaml, YamlLoader};

pub struct LegacyNode {
    pub process: JormungandrProcess,
    pub progress_bar: ProgressBarController,
    pub node_settings: LegacySettings,
}

impl LegacyNode {
    pub fn alias(&self) -> NodeAlias {
        self.process.alias()
    }

    pub fn status(&self) -> Status {
        self.process.status(&StartupVerificationMode::Rest)
    }

    pub fn check_running(&self) -> bool {
        self.status() == Status::Running
    }

    pub fn progress_bar(&self) -> &ProgressBarController {
        &self.progress_bar
    }

    pub fn log(&self, info: &str) {
        self.progress_bar.log_info(info);
    }

    pub fn genesis_block_hash(&self) -> Result<HeaderId> {
        Ok(self.process.grpc().get_genesis_block_hash())
    }

    pub fn legacy_rest(&self) -> BackwardCompatibleRest {
        BackwardCompatibleRest::new(self.process.rest_address().to_string(), Default::default())
    }

    pub fn rest(&self) -> JormungandrRest {
        self.process.rest()
    }

    pub fn stats(&self) -> Result<Yaml> {
        let stats = self.legacy_rest().stats()?;
        let docs = YamlLoader::load_from_str(&stats)?;
        Ok(docs.get(0).unwrap().clone())
    }

    pub fn log_stats(&self) {
        self.progress_bar
            .log_info(format!("node stats ({:?})", self.stats()));
    }

    pub fn wait_for_bootstrap(&self) -> Result<()> {
        self.process
            .wait_for_bootstrap(&StartupVerificationMode::Rest, Duration::from_secs(150))
            .map_err(|e| Error::NodeFailedToBootstrap {
                alias: self.alias(),
                e,
            })
    }

    pub fn wait_for_shutdown(&mut self) -> Result<Option<ExitStatus>> {
        self.process
            .wait_for_shutdown(Duration::from_secs(30))
            .map_err(|e| Error::NodeFailedToShutdown {
                alias: self.alias(),
                e,
            })
    }

    #[allow(deprecated)]
    fn ports_are_opened(&self) -> bool {
        self.port_opened(self.node_settings.config.rest.listen.port())
            && self.port_opened(
                multiaddr::to_tcp_socket_addr(&self.node_settings.config.p2p.public_address)
                    .unwrap()
                    .port(),
            )
    }

    fn port_opened(&self, port: u16) -> bool {
        use std::net::TcpListener;
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    pub fn logger(&self) -> &JormungandrLogger {
        &self.process.logger
    }

    pub fn is_up(&self) -> bool {
        matches!(self.status(), Status::Running)
    }

    pub fn shutdown(&mut self) -> Result<Option<ExitStatus>> {
        let message = self.rest().shutdown()?;
        if message.is_empty() {
            self.progress_bar.log_info("shuting down..");
            self.wait_for_shutdown()
        } else {
            Err(Error::ShutdownProcedure {
                alias: self.alias(),
                message,
                logs: self.logger().get_lines_as_string(),
            })
        }
    }

    pub fn capture_logs(&mut self) {
        let stderr = self.process.child.stderr.take().unwrap();
        let reader = BufReader::new(stderr);
        for line_result in reader.lines() {
            let line = line_result.expect("failed to read a line from log output");
            self.progress_bar.log_info(&line);
        }
    }

    pub fn progress_bar_start(&self) {
        self.progress_bar.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.green} {wide_msg}")
                .tick_chars(style::TICKER),
        );
        self.progress_bar.enable_steady_tick(100);
        self.progress_bar.set_message(&format!(
            "{} {} ... [{}]",
            *style::icons::jormungandr,
            style::binary.apply_to(self.alias()),
            self.node_settings.config().rest.listen,
        ));
    }

    fn progress_bar_failure(&self) {
        self.progress_bar.finish_with_message(&format!(
            "{} {} {}",
            *style::icons::jormungandr,
            style::binary.apply_to(self.alias()),
            style::error.apply_to(*style::icons::failure)
        ));
    }

    fn progress_bar_success(&self) {
        self.progress_bar.finish_with_message(&format!(
            "{} {} {}",
            *style::icons::jormungandr,
            style::binary.apply_to(self.alias()),
            style::success.apply_to(*style::icons::success)
        ));
    }
}
