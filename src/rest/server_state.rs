use stats::SharedStats;

#[derive(Clone, Debug)]
pub struct ServerState {
    pub stats: SharedStats,
}
