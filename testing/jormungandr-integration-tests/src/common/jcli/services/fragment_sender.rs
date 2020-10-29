use super::{FragmentCheck, FragmentsCheck};
use crate::common::{
    jcli::{JCli, JCliCommand},
    jormungandr::JormungandrProcess,
};
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_lib::interfaces::FragmentLog;
use jortestkit::process::Wait;

pub struct FragmentSender<'a> {
    jcli: JCli,
    jormungandr: &'a JormungandrProcess,
}

impl<'a> FragmentSender<'a> {
    pub fn new(jcli: JCli, jormungandr: &'a JormungandrProcess) -> Self {
        Self { jcli, jormungandr }
    }

    pub fn send(self, transaction: &'a str) -> FragmentCheck {
        let id = self
            .jcli
            .rest()
            .v0()
            .message()
            .post(transaction, self.jormungandr.rest_uri());
        FragmentCheck::new(self.jcli, self.jormungandr, id)
    }

    pub fn send_many(self, transactions: &'a [String]) -> FragmentsCheck {
        for tx in transactions {
            self.jcli
                .rest()
                .v0()
                .message()
                .post(tx, self.jormungandr.rest_uri());
        }
        FragmentsCheck::new(self.jcli, self.jormungandr)
    }
}
