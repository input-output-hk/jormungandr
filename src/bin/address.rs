extern crate bech32;
extern crate chain_addr;
extern crate chain_crypto;
extern crate structopt;

use bech32::{Bech32, FromBase32, ToBase32};
use chain_addr::{Address, AddressReadable, Discrimination, Kind};
use chain_crypto::{AsymmetricKey, PublicKey};
use structopt::StructOpt;

fn main() {
    match Command::from_args() {
        Command::Info(info_args) => address_info(&info_args.address),
        Command::Single(single_args) => {
            if let Some(delegation) = single_args.delegation {
                mk_delegation(&single_args.key, single_args.testing, &delegation)
            } else {
                mk_single(&single_args.key, single_args.testing)
            }
        }
        Command::Account(account_args) => mk_account(&account_args.key, account_args.testing),
    }
}

fn address_info(address: &AddressReadable) {
    let Address(discrimination, kind) = address.to_address();
    match discrimination {
        Discrimination::Production => {
            println!("discrimination: production");
        }
        Discrimination::Test => {
            println!("discrimination: testing");
        }
    }

    match kind {
        Kind::Single(single) => println!("public key: {}", print_pub_key(single)),
        Kind::Account(account) => println!("account: {}", print_pub_key(account)),
        Kind::Group(pubk, groupk) => {
            println!("public key: {}", print_pub_key(pubk));
            println!("group key:  {}", print_pub_key(groupk));
        }
    }
}

fn mk_single(s: &str, testing: bool) {
    mk_address_1(s, testing, Kind::Single)
}

fn mk_delegation(s: &str, testing: bool, d: &str) {
    mk_address_2(s, d, testing, Kind::Group)
}

fn mk_account(s: &str, testing: bool) {
    mk_address_1(s, testing, Kind::Account)
}

fn mk_discrimination(testing: bool) -> Discrimination {
    if testing {
        Discrimination::Test
    } else {
        Discrimination::Production
    }
}

fn mk_kind_1<A, F>(s: &str, f: F) -> Kind
where
    F: FnOnce(PublicKey<A>) -> Kind,
    A: AsymmetricKey,
{
    f(parse_pub_key(s))
}

fn mk_kind_2<A1, A2, F>(s1: &str, s2: &str, f: F) -> Kind
where
    F: FnOnce(PublicKey<A1>, PublicKey<A2>) -> Kind,
    A1: AsymmetricKey,
    A2: AsymmetricKey,
{
    f(parse_pub_key(s1), parse_pub_key(s2))
}

fn mk_address(discrimination: Discrimination, kind: Kind) {
    let address = Address(discrimination, kind);
    println!("{}", AddressReadable::from_address(&address).to_string());
}

fn mk_address_1<A, F>(s: &str, testing: bool, f: F)
where
    F: FnOnce(PublicKey<A>) -> Kind,
    A: AsymmetricKey,
{
    let discrimination = mk_discrimination(testing);
    let kind = mk_kind_1(s, f);
    mk_address(discrimination, kind);
}

fn mk_address_2<A1, A2, F>(s: &str, d: &str, testing: bool, f: F)
where
    F: FnOnce(PublicKey<A1>, PublicKey<A2>) -> Kind,
    A1: AsymmetricKey,
    A2: AsymmetricKey,
{
    let discrimination = mk_discrimination(testing);
    let kind = mk_kind_2(s, d, f);
    mk_address(discrimination, kind);
}

fn print_pub_key<A: AsymmetricKey>(pk: PublicKey<A>) -> Bech32 {
    let hrp = A::PUBLIC_BECH32_HRP.to_string();
    Bech32::new(hrp, pk.to_base32()).unwrap()
}

fn parse_pub_key<A: AsymmetricKey>(s: &str) -> PublicKey<A> {
    let bech32: Bech32 = s.parse().unwrap();
    if bech32.hrp() == A::PUBLIC_BECH32_HRP {
        let pub_key_bytes = Vec::<u8>::from_base32(bech32.data()).unwrap();
        PublicKey::from_binary(&pub_key_bytes).unwrap()
    } else {
        panic!(
            "Invalid Key Type, received {} but was expecting {}",
            bech32.hrp(),
            A::PUBLIC_BECH32_HRP
        )
    }
}

/// Jormungandr address manipulation
///
/// Set of command to display and create addresses
#[derive(StructOpt)]
#[structopt(
    name = "address",
    rename_all = "kebab-case",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
enum Command {
    /// start jormungandr service and start participating to the network
    Info(InfoArgs),

    /// create an address from the single public key. This address does
    /// not have delegation
    Single(SingleArgs),

    /// create an address from the the single public key
    Account(AccountArgs),
}

#[derive(StructOpt)]
struct InfoArgs {
    /// An address, in bech32 format, to display the content
    /// and info that can be extracted from
    #[structopt(name = "ADDRESS")]
    address: AddressReadable,
}

#[derive(StructOpt)]
struct SingleArgs {
    /// A public key in bech32 encoding with the key type prefix
    #[structopt(name = "PUBLIC_KEY")]
    key: String,

    /// A public key in bech32 encoding with the key type prefix
    #[structopt(name = "DELEGATION_KEY")]
    delegation: Option<String>,

    /// set the discrimination type to testing (default is production)
    #[structopt(long = "testing")]
    testing: bool,
}

#[derive(StructOpt)]
struct AccountArgs {
    /// A public key in bech32 encoding with the key type prefix
    #[structopt(name = "PUBLIC_KEY")]
    key: String,

    /// set the discrimination type to testing (default is production)
    #[structopt(long = "testing")]
    testing: bool,
}
