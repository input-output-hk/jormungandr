mod add_account;
mod add_certificate;
mod add_input;
mod add_output;
mod add_witness;
mod auth;
mod common;
mod finalize;
mod info;
mod mk_witness;
mod new;
mod seal;
mod staging;

use self::staging::StagingKind;
use crate::jcli_app::certificate;
use crate::jcli_app::utils::error::CustomErrorFiller;
use crate::jcli_app::utils::{key_parser, output_format};
use chain_core::property::Serialize as _;
use chain_impl_mockchain as chain;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Transaction {
    /// create a new staging transaction. The transaction is initially
    /// empty.
    New(new::New),

    /// add UTxO input to the transaction
    AddInput(add_input::AddInput),
    /// add Account input to the transaction
    AddAccount(add_account::AddAccount),
    /// add output to the transaction
    AddOutput(add_output::AddOutput),
    /// add output to the finalized transaction
    AddWitness(add_witness::AddWitness),
    /// set a certificate to the Transaction. If there is already
    /// an extra certificate in the transaction it will be replaced
    /// with the new one.
    AddCertificate(add_certificate::AddCertificate),
    /// Lock a transaction and start adding witnesses
    Finalize(finalize::Finalize),
    /// Finalize the transaction
    Seal(seal::Seal),
    /// get the Fragment ID from the given 'sealed' transaction
    FragmentId(common::CommonTransaction),
    /// DEPRECATED: use 'data-for-witness' instead
    Id(common::CommonTransaction),
    /// get the data to sign from the given transaction
    /// (if the transaction is edited, the returned value will change)
    DataForWitness(common::CommonTransaction),
    /// display the info regarding a given transaction
    Info(info::Info),
    /// create witnesses
    MakeWitness(mk_witness::MkWitness),
    /// make auth
    Auth(auth::Auth),
    /// get the message format out of a sealed transaction
    ToMessage(common::CommonTransaction),
}

type StaticStr = &'static str;

custom_error! { pub Error
    StagingFileOpenFailed { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("could not open staging transaction file '{}'", path.display()) }},
    StagingFileReadFailed { source: bincode::ErrorKind, path: PathBuf }
        = @{{ let _ = source; format_args!("could not read staging transaction file '{}'", path.display()) }},
    StagingFileWriteFailed { source: bincode::ErrorKind, path: PathBuf }
        = @{{ let _ = source; format_args!("could not write staging transaction file '{}'", path.display()) }},
    SecretFileFailed { source: key_parser::Error }
        = @{{ format_args!("could not process secret file '{}'", source) }},
        /*
    SecretFileReadFailed { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("could not read secret file '{}'", path.display()) }},
    SecretFileMalformed { source: chain_crypto::bech32::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("could not decode secret file '{}'", path.display()) }},
        */
    WitnessFileReadFailed { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("could not read witness file '{}'", path.display()) }},
    WitnessFileWriteFailed { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("could not write witness file '{}'", path.display()) }},
    WitnessFileBech32Malformed { source: bech32::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("could not parse Bech32 in witness file '{}'", path.display()) }},
    WitnessFileBech32HrpInvalid { actual: String, expected: StaticStr, path: PathBuf }
        = @{{ format_args!("invalid Bech32 prefix in witness file, expected '{}', found '{}' in '{}'",
            expected, actual, path.display()) }},
    WitnessFileBech32EncodingFailed { source: bech32::Error } = "failed to encode witness as bech32",
    WitnessFileDeserializationFailed { source: chain_core::mempack::ReadError, path: PathBuf }
        = @{{ let _ = source; format_args!("could not parse data in witness file '{}'", path.display()) }},
    WitnessFileSerializationFailed { source: std::io::Error, filler: CustomErrorFiller }
        = "could not serialize witness data",
    InfoFileWriteFailed { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("could not write info file '{}'", path.display()) }},
    OutputFormatFailed { source: output_format::Error } = "formatting output failed",

    TxKindToAddExtraInvalid { kind: StagingKind } = "adding certificate to {kind} transaction is not valid",
    TxKindToAddInputInvalid { kind: StagingKind } = "adding input to {kind} transaction is not valid",
    TxKindToAddOutputInvalid { kind: StagingKind } = "adding output to {kind} transaction is not valid",
    TxKindToAddWitnessInvalid { kind: StagingKind } = "adding witness to {kind} transaction is not valid",
    TxKindToSealInvalid { kind: StagingKind } = "sealing {kind} transaction is not valid",
    TxKindToFinalizeInvalid { kind: StagingKind } = "finalizing {kind} transaction is not valid",
    TxKindToGetMessageInvalid { kind: StagingKind } = "cannot get message from transaction in {kind} state",

    TooManyWitnessesToAddWitness { actual: usize, max: usize }
        = "too many witnesses in transaction to add another: {actual}, maximum is {max}",
    WitnessCountToSealInvalid { actual: usize, expected: usize }
        = "invalid number of witnesses in transaction to seal: {actual}, should be {expected}",
    AccountAddressSingle = "invalid input account, this is a UTxO address",
    AccountAddressGroup = "invalid input account, this is a UTxO address with delegation",
    AccountAddressMultisig = "invalid input account, this is a multisig account address",
    AddingWitnessToFinalizedTxFailed { filler: CustomErrorFiller }
        = "could not add witness to finalized transaction",
    GeneratedTxBuildingFailed { filler: CustomErrorFiller }
        = "generated transaction building failed",
    TxFinalizationFailed { source: chain::transaction::Error }
        = "transaction finalization failed",
    GeneratedTxTypeUnexpected = "unexpected generated transaction type",
    MessageSerializationFailed { source: std::io::Error, filler: CustomErrorFiller }
        = "serialization of message to bytes failed",
    InfoCalculationFailed { source: chain::value::ValueError } = "calculation of info failed",
    FeeCalculationFailed = "fee calculation failed",
    InfoExpectedSingleAccount = "expected a single account, multisig is not supported yet",
    MakeWitnessLegacyUtxoUnsupported = "making legacy UTxO witness unsupported",
    MakeWitnessAccountCounterMissing = "making account witness requires passing spending counter",
    TxDoesntNeedPayloadAuth = "transaction type doesn't need payload authentification",
    TxNeedPayloadAuth = "transaction type need payload authentification",
    NoSigningKeys = "No signing keys specified (use -k or --key to specify)",
    ExpectingOnlyOneSigningKey { got: usize }
        = "expecting only one signing keys but got {got}",
    CertificateError { error: certificate::Error } = "certificate error {error}",
}

/*
impl From<key_parser::Error> for Error {
    fn from(kp: key_parser::Error) -> Self {
        Error::SecretFileFailed { source: kp }
    }
}
*/

impl Transaction {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Transaction::New(new) => new.exec(),
            Transaction::AddInput(add_input) => add_input.exec(),
            Transaction::AddAccount(add_account) => add_account.exec(),
            Transaction::AddOutput(add_output) => add_output.exec(),
            Transaction::AddWitness(add_witness) => add_witness.exec(),
            Transaction::AddCertificate(add_certificate) => add_certificate.exec(),
            Transaction::Finalize(finalize) => finalize.exec(),
            Transaction::Seal(seal) => seal.exec(),
            Transaction::FragmentId(common) => display_fragment_id(common),
            Transaction::Id(common) => display_id(common),
            Transaction::DataForWitness(common) => display_data_for_witness(common),
            Transaction::Info(info) => info.exec(),
            Transaction::MakeWitness(mk_witness) => mk_witness.exec(),
            Transaction::Auth(auth) => auth.exec(),
            Transaction::ToMessage(common) => display_message(common),
        }
    }
}

fn display_id(common: common::CommonTransaction) -> Result<(), Error> {
    eprintln!("DEPRECATED: use 'data-for-witness' instead");
    display_data_for_witness(common)
}

fn display_data_for_witness(common: common::CommonTransaction) -> Result<(), Error> {
    let id = common.load()?.transaction_sign_data_hash();
    println!("{}", id);
    Ok(())
}

fn display_fragment_id(common: common::CommonTransaction) -> Result<(), Error> {
    let id = common.load()?.fragment()?.hash();
    println!("{}", id);
    Ok(())
}

fn display_message(common: common::CommonTransaction) -> Result<(), Error> {
    let message = common.load()?.fragment()?;
    let bytes: Vec<u8> =
        message
            .serialize_as_vec()
            .map_err(|source| Error::MessageSerializationFailed {
                source,
                filler: CustomErrorFiller,
            })?;
    println!("{}", hex::encode(&bytes));
    Ok(())
}
