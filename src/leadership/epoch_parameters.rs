use crate::blockcfg::{Epoch, Ledger, LedgerParameters, LedgerStaticParameters};

/// parameters passed to the leadership [`Process`]
/// and then broadcasted to every leader [`Task`].
///
/// [`Process`]: ./struct.Process.html
/// [`Task`]: ./struct.Task.html
pub struct EpochParameters {
    pub epoch: Epoch,
    pub ledger_static_parameters: LedgerStaticParameters,
    pub ledger_parameters: LedgerParameters,

    pub ledger_reference: Ledger,
}
