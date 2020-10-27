#![allow(dead_code)]

use crate::common::jcli::JCli;
use assert_fs::{prelude::*, TempDir};
use jormungandr_lib::interfaces::TaxType;
use jortestkit::file;
use std::path::{Path, PathBuf};

pub struct CertificateBuilder {
    jcli: JCli,
}

impl CertificateBuilder {
    pub fn new(jcli: JCli) -> Self {
        Self { jcli }
    }
    pub fn new_signed_stake_pool_cert(
        &self,
        pool_kes_pk: &str,
        pool_vrf_pk: &str,
        stake_key_file: &Path,
        start_validity: u32,
        management_threshold: u32,
        owner_pk: &str,
        tax_type: Option<TaxType>,
    ) {
        let temp_dir = TempDir::new().unwrap();

        let stake_pool_cert = self.jcli.certificate().new_stake_pool_registration(
            &pool_kes_pk,
            &pool_vrf_pk,
            start_validity,
            management_threshold,
            owner_pk,
            tax_type,
        );
        let stake_pool_cert_file = temp_dir.child("stake_pool.cert");
        stake_pool_cert_file.write_str(&stake_pool_cert).unwrap();

        let stake_pool_signcert_file = temp_dir.child("stake_pool.signcert");
        self.jcli.certificate().sign(
            &stake_key_file,
            stake_pool_cert_file.path(),
            stake_pool_signcert_file.path(),
        );
    }

    pub fn new_signed_stake_pool_delegation(
        self,
        stake_pool_id: &str,
        stake_key_pub: &str,
        stake_key_file: &Path,
    ) -> String {
        let temp_dir = TempDir::new().unwrap();

        let stake_delegation_cert = self
            .jcli
            .certificate()
            .new_stake_delegation(stake_pool_id, stake_key_pub);

        let stake_delegation_cert_file = temp_dir.child("stake_delegation.cert");
        stake_delegation_cert_file
            .write_str(&stake_delegation_cert)
            .unwrap();
        let stake_delegation_signcert_file = temp_dir.child("stake_delegation.signcert");

        self.jcli.certificate().sign(
            &stake_key_file,
            stake_delegation_cert_file.path(),
            stake_delegation_signcert_file.path(),
        );
        file::read_file(stake_delegation_signcert_file.path())
    }

    pub fn new_signed_vote_plan<P: AsRef<Path>, Q: AsRef<Path>>(
        self,
        proposal_file: P,
        stake_key_file: Q,
    ) -> PathBuf {
        let temp_dir = TempDir::new().unwrap();
        let cert = self.jcli.certificate().new_vote_plan(proposal_file);

        let cert_file = temp_dir.child("vote_plan.cert");
        cert_file.write_str(&cert).unwrap();

        let signcert_file = temp_dir.child("vote_plan.signcert");
        self.jcli
            .certificate()
            .sign(&stake_key_file, cert_file.path(), signcert_file.path());
        PathBuf::from(signcert_file.path())
    }
}
