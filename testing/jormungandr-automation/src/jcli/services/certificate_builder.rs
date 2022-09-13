#![allow(dead_code)]

use crate::jcli::JCli;
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

    pub fn new_signed_stake_pool_cert(self) -> SignedStakePoolCertBuilder {
        SignedStakePoolCertBuilder::new(self.jcli)
    }

    pub fn new_signed_stake_pool_delegation(
        self,
        stake_pool_id: &str,
        stake_key_pub: &str,
        stake_key_file: &Path,
    ) -> Result<String, std::io::Error> {
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
            stake_key_file,
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
        let temp_dir = TempDir::new().unwrap().into_persistent();
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

pub struct SignedStakePoolCertBuilder {
    jcli: JCli,
    pool_kes_pk: String,
    pool_vrf_pk: String,
    stake_key_file: PathBuf,
    start_validity: u32,
    management_threshold: u32,
    owner_pk: String,
    tax_type: Option<TaxType>,
}

impl SignedStakePoolCertBuilder {
    pub fn new(jcli: JCli) -> Self {
        Self {
            jcli,
            pool_kes_pk: "".to_owned(),
            pool_vrf_pk: "".to_string(),
            stake_key_file: PathBuf::new(),
            start_validity: 0u32,
            management_threshold: 0u32,
            owner_pk: "".to_string(),
            tax_type: None,
        }
    }

    pub fn pool_kes_pk<S: Into<String>>(&mut self, pool_kes_pk: S) -> &mut Self {
        self.pool_kes_pk = pool_kes_pk.into();
        self
    }

    pub fn pool_vrf_pk<S: Into<String>>(&mut self, pool_vrf_pk: S) -> &mut Self {
        self.pool_vrf_pk = pool_vrf_pk.into();
        self
    }

    pub fn owner_pk<S: Into<String>>(&mut self, owner_pk: S) -> &mut Self {
        self.owner_pk = owner_pk.into();
        self
    }

    pub fn stake_key_file<P: AsRef<Path>>(&mut self, stake_key_file: P) -> &mut Self {
        self.stake_key_file = stake_key_file.as_ref().to_path_buf();
        self
    }

    pub fn start_validity(&mut self, start_validity: u32) -> &mut Self {
        self.start_validity = start_validity;
        self
    }

    pub fn management_threshold(&mut self, management_threshold: u32) -> &mut Self {
        self.management_threshold = management_threshold;
        self
    }

    pub fn tax_type(&mut self, tax_type: TaxType) -> &mut Self {
        self.tax_type = Some(tax_type);
        self
    }

    pub fn build(self) {
        let temp_dir = TempDir::new().unwrap();

        let stake_pool_cert = self.jcli.certificate().new_stake_pool_registration(
            &self.pool_kes_pk,
            &self.pool_vrf_pk,
            self.start_validity,
            self.management_threshold,
            &self.owner_pk,
            self.tax_type,
        );
        let stake_pool_cert_file = temp_dir.child("stake_pool.cert");
        stake_pool_cert_file.write_str(&stake_pool_cert).unwrap();

        let stake_pool_signcert_file = temp_dir.child("stake_pool.signcert");
        self.jcli.certificate().sign(
            &self.stake_key_file,
            stake_pool_cert_file.path(),
            stake_pool_signcert_file.path(),
        );
    }
}
