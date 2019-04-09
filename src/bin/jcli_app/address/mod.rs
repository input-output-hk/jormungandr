use bech32::{Bech32, FromBase32, ToBase32};
use chain_addr::{AddressReadable, Discrimination, Kind};
use chain_crypto::{AsymmetricKey, Ed25519Extended, PublicKey};
use jcli_app::utils::key_parser::parse_pub_key;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "address", rename_all = "kebab-case")]
pub enum Address {
    /// start jormungandr service and start participating to the network
    Info(InfoArgs),

    /// create an address from the single public key. This address does
    /// not have delegation
    Single(SingleArgs),

    /// create an address from the the single public key
    Account(AccountArgs),
}

#[derive(StructOpt)]
pub struct InfoArgs {
    /// An address, in bech32 format, to display the content
    /// and info that can be extracted from
    #[structopt(name = "ADDRESS")]
    address: AddressReadable,
}

#[derive(StructOpt)]
pub struct SingleArgs {
    /// A public key in bech32 encoding with the key type prefix
    #[structopt(name = "PUBLIC_KEY", parse(from_str = "parse_pub_key"))]
    key: PublicKey<Ed25519Extended>,

    /// A public key in bech32 encoding with the key type prefix
    #[structopt(name = "DELEGATION_KEY", parse(from_str = "parse_pub_key"))]
    delegation: Option<PublicKey<Ed25519Extended>>,

    /// set the discrimination type to testing (default is production)
    #[structopt(long = "testing")]
    testing: bool,
}

#[derive(StructOpt)]
pub struct AccountArgs {
    /// A public key in bech32 encoding with the key type prefix
    #[structopt(name = "PUBLIC_KEY", parse(from_str = "parse_pub_key"))]
    key: PublicKey<Ed25519Extended>,

    /// set the discrimination type to testing (default is production)
    #[structopt(long = "testing")]
    testing: bool,
}

impl Address {
    pub fn exec(self) {
        match self {
            Address::Info(info_args) => address_info(&info_args.address),
            Address::Single(single_args) => {
                if let Some(delegation) = single_args.delegation {
                    mk_delegation(single_args.key, single_args.testing, delegation)
                } else {
                    mk_single(single_args.key, single_args.testing)
                }
            }
            Address::Account(account_args) => mk_account(account_args.key, account_args.testing),
        }
    }
}

fn address_info(address: &AddressReadable) {
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
        Kind::Single(single) => println!("public key: {}", print_pub_key(single)),
        Kind::Account(account) => println!("account: {}", print_pub_key(account)),
        Kind::Group(pubk, groupk) => {
            println!("public key: {}", print_pub_key(pubk));
            println!("group key:  {}", print_pub_key(groupk));
        }
    }
}

fn mk_single(s: PublicKey<Ed25519Extended>, testing: bool) {
    mk_address_1(s, testing, Kind::Single)
}

fn mk_delegation(s: PublicKey<Ed25519Extended>, testing: bool, d: PublicKey<Ed25519Extended>) {
    mk_address_2(s, d, testing, Kind::Group)
}

fn mk_account(s: PublicKey<Ed25519Extended>, testing: bool) {
    mk_address_1(s, testing, Kind::Account)
}

fn mk_discrimination(testing: bool) -> Discrimination {
    if testing {
        Discrimination::Test
    } else {
        Discrimination::Production
    }
}

fn mk_address(discrimination: Discrimination, kind: Kind) {
    let address = chain_addr::Address(discrimination, kind);
    println!("{}", AddressReadable::from_address(&address).to_string());
}

fn mk_address_1<A, F>(s: PublicKey<A>, testing: bool, f: F)
where
    F: FnOnce(PublicKey<A>) -> Kind,
    A: AsymmetricKey,
{
    let discrimination = mk_discrimination(testing);
    let kind = f(s);
    mk_address(discrimination, kind);
}

fn mk_address_2<A1, A2, F>(s: PublicKey<A1>, d: PublicKey<A2>, testing: bool, f: F)
where
    F: FnOnce(PublicKey<A1>, PublicKey<A2>) -> Kind,
    A1: AsymmetricKey,
    A2: AsymmetricKey,
{
    let discrimination = mk_discrimination(testing);
    let kind = f(s, d);
    mk_address(discrimination, kind);
}

fn print_pub_key<A: AsymmetricKey>(pk: PublicKey<A>) -> Bech32 {
    let hrp = A::PUBLIC_BECH32_HRP.to_string();
    Bech32::new(hrp, pk.to_base32()).unwrap()
}
