use super::{FragmentCheck, FragmentsCheck};
use crate::testing::common::{jcli::JCli, jormungandr::JormungandrProcess};

pub struct FragmentSender<'a> {
    jcli: JCli,
    jormungandr: &'a JormungandrProcess,
}

impl<'a> FragmentSender<'a> {
    pub fn new(jcli: JCli, jormungandr: &'a JormungandrProcess) -> Self {
        Self { jcli, jormungandr }
    }

    pub fn send(self, transaction: &'a str) -> FragmentCheck {
        let summary = self
            .jcli
            .rest()
            .v0()
            .message()
            .post(transaction, self.jormungandr.rest_uri());

        let id = if summary.accepted.len() == 1 {
            summary.accepted[0]
        } else if summary.rejected.len() == 1 {
            summary.rejected[0].id
        } else {
            panic!("Single transaction was sent but multiple or no processing results found");
        };

        FragmentCheck::new(self.jcli, self.jormungandr, id, summary)
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
