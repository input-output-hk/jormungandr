use crate::jcli_lib::utils::key_parser::parse_pub_key;
use chain_addr::{AddressReadable, Discrimination, Kind};
use chain_crypto::{bech32::Bech32 as _, AsymmetricPublicKey, Ed25519, PublicKey};
use structopt::StructOpt;
use thiserror::Error;

#[derive(StructOpt)]
#[structopt(name = "address", rename_all = "kebab-case")]
pub enum Address {
    /// Display the content and info of a bech32 formatted address.
    Info(InfoArgs),

    /// Create an address from a single public key. This address does
    /// not have delegation.
    Single(SingleArgs),

    /// Create an account address from a single public key.
    Account(AccountArgs),
}

#[derive(StructOpt)]
pub struct InfoArgs {
    /// An address, in bech32 format, to display the content
    /// and info that can be extracted from.
    #[structopt(name = "ADDRESS")]
    address: AddressReadable,
}

#[derive(StructOpt)]
pub struct DiscriminationData {
    /// Set the discrimination type to testing (default is production).
    #[structopt(long = "testing")]
    testing: bool,

    /// Set the prefix to use to describe the address. This is only available
    /// on the human readable representation of the address and will not be
    /// used or checked by the node.
    #[structopt(long = "prefix", default_value = "ca")]
    prefix: String,
}

#[derive(StructOpt)]
pub struct SingleArgs {
    /// A public key in bech32 encoding with the key type prefix.
    #[structopt(name = "PUBLIC_KEY", parse(try_from_str = parse_pub_key))]
    key: PublicKey<Ed25519>,

    /// A public key in bech32 encoding with the key type prefix.
    #[structopt(name = "DELEGATION_KEY", parse(try_from_str = parse_pub_key))]
    delegation: Option<PublicKey<Ed25519>>,

    #[structopt(flatten)]
    discrimination_data: DiscriminationData,
}

#[derive(StructOpt)]
pub struct AccountArgs {
    /// A public key in bech32 encoding with the key type prefix.
    #[structopt(name = "PUBLIC_KEY", parse(try_from_str = parse_pub_key))]
    key: PublicKey<Ed25519>,

    #[structopt(flatten)]
    discrimination_data: DiscriminationData,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("multisig addresses are not supported")]
    MultisigAddressNotSupported,
}

impl Address {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Address::Info(info_args) => address_info(&info_args.address)?,
            Address::Single(single_args) => {
                if let Some(delegation) = single_args.delegation {
                    mk_delegation(
                        &single_args.discrimination_data.prefix,
                        single_args.key,
                        single_args.discrimination_data.testing,
                        delegation,
                    )
                } else {
                    mk_single(
                        &single_args.discrimination_data.prefix,
                        single_args.key,
                        single_args.discrimination_data.testing,
                    )
                }
            }
            Address::Account(account_args) => mk_account(
                &account_args.discrimination_data.prefix,
                account_args.key,
                account_args.discrimination_data.testing,
            ),
        }
        Ok(())
    }
}

fn address_info(address: &AddressReadable) -> Result<(), Error> {
    let chain_addr::Address(discrimination, kind) = address.to_address();
    match discrimination {
        Discrimination::Production => {
            println!("discrimination: production");
        }
        Discrimination::Test => {
            println!("discrimination: testing");
        }
    }

    match kind {
        Kind::Single(single) => println!("public key: {}", single.to_bech32_str()),
        Kind::Account(account) => println!("account: {}", account.to_bech32_str()),
        Kind::Multisig(_) => return Err(Error::MultisigAddressNotSupported),
        Kind::Group(pubk, groupk) => {
            println!("public key: {}", pubk.to_bech32_str());
            println!("group key:  {}", groupk.to_bech32_str());
        }
        Kind::Script(id) => println!("script identifier: {}", hex::encode(id)),
    }
    Ok(())
}

fn mk_single(prefix: &str, s: PublicKey<Ed25519>, testing: bool) {
    mk_address_1(prefix, s, testing, Kind::Single)
}

fn mk_delegation(prefix: &str, s: PublicKey<Ed25519>, testing: bool, d: PublicKey<Ed25519>) {
    mk_address_2(prefix, s, d, testing, Kind::Group)
}

fn mk_account(prefix: &str, s: PublicKey<Ed25519>, testing: bool) {
    mk_address_1(prefix, s, testing, Kind::Account)
}

fn mk_discrimination(testing: bool) -> Discrimination {
    if testing {
        Discrimination::Test
    } else {
        Discrimination::Production
    }
}

fn mk_address(prefix: &str, discrimination: Discrimination, kind: Kind) {
    let address = chain_addr::Address(discrimination, kind);
    println!("{}", AddressReadable::from_address(prefix, &address));
}

fn mk_address_1<A, F>(prefix: &str, s: PublicKey<A>, testing: bool, f: F)
where
    F: FnOnce(PublicKey<A>) -> Kind,
    A: AsymmetricPublicKey,
{
    let discrimination = mk_discrimination(testing);
    let kind = f(s);
    mk_address(prefix, discrimination, kind);
}

fn mk_address_2<A1, A2, F>(prefix: &str, s: PublicKey<A1>, d: PublicKey<A2>, testing: bool, f: F)
where
    F: FnOnce(PublicKey<A1>, PublicKey<A2>) -> Kind,
    A1: AsymmetricPublicKey,
    A2: AsymmetricPublicKey,
{
    let discrimination = mk_discrimination(testing);
    let kind = f(s, d);
    mk_address(prefix, discrimination, kind);
}
