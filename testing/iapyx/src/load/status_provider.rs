use crate::WalletBackend;
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_lib::{
    interfaces::{FragmentLog, FragmentStatus},
    time::SystemTime,
};
use jortestkit::load::RequestStatusProvider;
use jortestkit::load::{Id, Status};

pub struct VoteStatusProvider {
    backend: WalletBackend,
}

impl VoteStatusProvider {
    pub fn new(backend_address: String) -> Self {
        let mut backend = WalletBackend::new(backend_address, Default::default());
        backend.disable_logs();
        Self { backend }
    }
}

impl RequestStatusProvider for VoteStatusProvider {
    fn get_statuses(&self, ids: &[Id]) -> Vec<Status> {
        let fragment_logs = self.backend.fragment_logs().unwrap();
        fragment_logs
            .iter()
            .filter(|(id, _)| ids.contains(&id.to_string()))
            .map(|(id, fragment_log)| into_status(fragment_log, id))
            .collect()
    }
}

fn into_status(fragment_log: &FragmentLog, id: &FragmentId) -> Status {
    match fragment_log.status() {
        FragmentStatus::Pending => {
            let duration = SystemTime::now()
                .duration_since(*fragment_log.received_at())
                .unwrap();
            Status::new_pending(duration.into(), id.to_string())
        }
        FragmentStatus::Rejected { reason } => {
            let duration = fragment_log
                .last_updated_at()
                .duration_since(*fragment_log.received_at())
                .unwrap();
            Status::new_failure(duration.into(), id.to_string(), reason.to_string())
        }
        FragmentStatus::InABlock { .. } => {
            let duration = fragment_log
                .last_updated_at()
                .duration_since(*fragment_log.received_at())
                .unwrap();
            Status::new_success(duration.into(), id.to_string())
        }
    }
}
