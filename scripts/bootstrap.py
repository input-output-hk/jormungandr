#!/bin/env python3

import argparse
import shutil
from typing import Tuple
import subprocess
import tempfile
import time
import os
from pathlib import Path
import sys

ADDRESS_TYPE = "--testing"
INITIAL_FUNDS = 1000000000000
REST_HOST = "127.0.0.1"

parser = argparse.ArgumentParser(
    description="Configure a single-node jormungandr network"
)
parser.add_argument(
    "--rest-port", type=int, default=8443, help="The port for REST API to listen on"
)
parser.add_argument("--bft", action="store_true", help="Use BFT consensus mode")
parser.add_argument(
    "--genesis-praos", action="store_true", help="Use Genesis-Praos consensus mode"
)
parser.add_argument("--startup-script", action="store_true", help="Add startup script")
parser.add_argument(
    "--secret-path",
    type=str,
    default=os.getcwd(),
    help="The path to write secret files",
)
parser.add_argument(
    "--config-path",
    type=str,
    default=os.getcwd(),
    help="The path to write config files",
)
parser.add_argument(
    "--data-path", type=str, default=os.getcwd(), help="The path to blockchain storage"
)
parser.add_argument(
    "--slot-duration", type=int, default=10, help="Duration of a slot in seconds"
)
parser.add_argument(
    "--slots-per-epoch", type=int, default=5000, help="Number of slots in each epoch"
)
parser.add_argument(
    "--no-colors", action="store_true", help="Disable shell colours in stdout"
)
parser.add_argument(
    "--overwrite",
    action="store_true",
    help="Overwrite existing blockchain storage files",
)
parser.add_argument("--jcli", type=str, default="jcli", help="Path to jcli")
parser.add_argument(
    "--jormungandr", type=str, default="jormungandr", help="Path to jormungandr"
)

args = parser.parse_args()

if args.bft and args.genesis_praos:
    raise RuntimeError("You must select one consensus to use")
elif args.bft:
    consensus = "bft"
elif args.genesis_praos:
    consensus = "genesis_praos"
else:
    raise RuntimeError(
        "You must specify a consensus to use with --bft or --genesis-praos"
    )

jcli = args.jcli
jormungandr = args.jormungandr

if shutil.which(jormungandr) is None:
    raise RuntimeError("{} not found".format(jormungandr))
if shutil.which(jcli) is None:
    raise RuntimeError("{} not found".format(jcli))

config_path = Path(args.config_path)
secret_path = Path(args.secret_path)
data_path = Path(args.data_path)

storage = Path(data_path / "jormungandr-storage-test")

try:
    if os.listdir(str(storage)):
        if args.overwrite:
            shutil.rmtree(storage)
        else:
            print(
                "error: directory {} contains blocks.sqlite already, use --overwrite to overwrite".format(
                    str(storage)
                )
            )
            sys.exit(1)
except FileNotFoundError:
    pass

if args.no_colors:
    color_red = ""
    color_white = ""
    color_green = ""
    color_blue = ""
else:
    color_red = "\033[0;31m"
    color_green = "\033[0;32m"
    color_blue = "\033[0;33m"
    color_white = "\033[0m"


def make_key(key_type: str) -> Tuple[str, str]:
    secret = subprocess.run(
        [jcli, "key", "generate", "--type={}".format(key_type)],
        capture_output=True,
        check=True,
    ).stdout
    public = subprocess.run(
        [jcli, "key", "to-public"], input=secret, capture_output=True, check=True
    ).stdout
    return secret.decode("utf-8").strip(), public.decode("utf-8").strip()


def make_key_and_address(key_type: str) -> Tuple[str, str, str]:
    secret, public = make_key(key_type)
    address = (
        subprocess.run(
            [jcli, "address", "account", ADDRESS_TYPE, public],
            capture_output=True,
            check=True,
        )
        .stdout.decode("utf-8")
        .strip()
    )
    return secret, public, address


def sign_certificate(certificate: bytes, secret_key: str) -> bytes:
    key_file = tempfile.NamedTemporaryFile(mode="w")
    key_file.write(secret_key)
    key_file.flush()
    certificate_signed = subprocess.run(
        [jcli, "certificate", "sign", "--key", key_file.name],
        input=certificate,
        capture_output=True,
        check=True,
    ).stdout
    return certificate_signed


faucet_secret_key, faucet_public_key, faucet_address = make_key_and_address(
    "Ed25519Extended"
)
fixed_secret_key, fixed_public_key, fixed_address = make_key_and_address(
    "Ed25519Extended"
)
leader_secret_key, leader_public_key = make_key("Ed25519")
vrf_secret_key, vrf_public_key = make_key("EllipticCurve2hashDh")
kes_secret_key, kes_public_key = make_key("SumEd25519_12")

stake_key_secret = faucet_secret_key
stake_key_public = faucet_public_key

stake_pool_certificate_unsigned = subprocess.run(
    [
        jcli,
        "certificate",
        "new",
        "stake-pool-registration",
        "--management-threshold",
        "1",
        "--start-validity",
        "0",
        "--owner",
        leader_public_key,
        "--kes-key",
        kes_public_key,
        "--tax-fixed",
        "10",
        "--tax-limit",
        "1000000000",
        "--tax-ratio",
        "1/10",
        "--reward-account",
        faucet_address,
        "--vrf-key",
        vrf_public_key,
    ],
    capture_output=True,
    check=True,
).stdout
stake_pool_certificate = sign_certificate(
    stake_pool_certificate_unsigned, leader_secret_key
)

stake_pool_id = (
    subprocess.run(
        [jcli, "certificate", "show", "stake-pool-id"],
        input=stake_pool_certificate,
        capture_output=True,
        check=True,
    )
    .stdout.decode("utf-8")
    .strip()
)

stake_delegation1_unsigned = subprocess.run(
    [jcli, "certificate", "new", "stake-delegation", faucet_public_key, stake_pool_id],
    capture_output=True,
    check=True,
).stdout
stake_delegation1 = sign_certificate(stake_delegation1_unsigned, faucet_secret_key)

stake_delegation2_unsigned = subprocess.run(
    [jcli, "certificate", "new", "stake-delegation", fixed_public_key, stake_pool_id],
    capture_output=True,
    check=True,
).stdout
stake_delegation2 = sign_certificate(stake_delegation2_unsigned, fixed_secret_key)

genesis_yaml = """\
blockchain_configuration:
  block0_date: {date}
  discrimination: test
  slots_per_epoch: {slots_per_epoch}
  slot_duration: {slot_duration}
  epoch_stability_depth: 10
  consensus_genesis_praos_active_slot_coeff: 0.1
  consensus_leader_ids:
    - {leader_public_key}
  linear_fees:
    constant: 10
    coefficient: 0
    certificate: 0
  block0_consensus: {consensus}
  kes_update_speed: 43200 # 12 hours
  total_reward_supply: 100000000000000
  reward_parameters:
    halving:
      constant: 1000
      ratio: "1/1"
      epoch_start: 0
      epoch_rate: 1
  treasury: 0
  treasury_parameters:
    fixed: 0
    ratio: "0/1"
initial:
  - fund:
      - address: {faucet_address}
        value: {initial_funds}
      - address: {fixed_address}
        value: {initial_funds}
  - cert: {stake_pool_certificate}
  - cert: {stake_delegation1}
  - cert: {stake_delegation2}
""".format(
    date=int(time.time()),
    slots_per_epoch=args.slots_per_epoch,
    slot_duration=args.slot_duration,
    leader_public_key=leader_public_key,
    consensus=consensus,
    faucet_address=faucet_address,
    fixed_address=fixed_address,
    stake_pool_certificate=stake_pool_certificate.decode("utf-8").strip(),
    stake_delegation1=stake_delegation1.decode("utf-8").strip(),
    stake_delegation2=stake_delegation2.decode("utf-8").strip(),
    initial_funds=INITIAL_FUNDS,
)

genesis_yaml_path = Path(config_path / "genesis.yaml")

with open(genesis_yaml_path, "w") as genesis_file:
    genesis_file.write(genesis_yaml)

pool_secret1_yaml = """\
genesis:
  sig_key: {kes_secret_key}
  vrf_key: {vrf_secret_key}
  node_id: {stake_pool_id}
bft:
  signing_key: {leader_secret_key}
""".format(
    kes_secret_key=kes_secret_key,
    vrf_secret_key=vrf_secret_key,
    stake_pool_id=stake_pool_id,
    leader_secret_key=leader_secret_key,
)

pool_secret_path = Path(secret_path / "pool-secret1.yaml")

with open(pool_secret_path, "w") as pool_secret1_file:
    pool_secret1_file.write(pool_secret1_yaml)

rest_listen = "{}:{}".format(REST_HOST, args.rest_port)

config_yaml = """\
storage: "{storage}"

rest:
  listen: "{rest_listen}"

p2p:
  trusted_peers: []
  public_address: "/ip4/{rest_host}/tcp/8299"
""".format(
    storage=storage.as_posix(), rest_listen=rest_listen, rest_host=REST_HOST
)

config_yaml_path = Path(config_path / "config.yaml")

with open(config_yaml_path, "w") as config_file:
    config_file.write(config_yaml)

block0_path = Path(config_path / "block-0.bin")

subprocess.run(
    [
        jcli,
        "genesis",
        "encode",
        "--input",
        str(genesis_yaml_path),
        "--output",
        str(block0_path),
    ],
    check=True,
)

if args.startup_script:
    with open(Path(Path(os.getcwd() / "start-jormungandr.sh")), "w") as startup_script:
        startup_script.write(
            "{jormungandr} --genesis-block {genesis_path} --config {config_path} --secret {secret_path}".format(
                jormungandr=jormungandr,
                genesis_path=block0_path,
                config_path=config_yaml_path,
                secret_path=pool_secret_path,
            )
        )

jcli_version = (
    subprocess.run([jcli, "--version"], capture_output=True, check=True)
    .stdout.decode("utf-8")
    .strip()
)
jormungandr_version = (
    subprocess.run([jormungandr, "--version"], capture_output=True, check=True)
    .stdout.decode("utf-8")
    .strip()
)

print(
    """\
########################################################
* Consensus       : {red}{consensus}{white}
* REST Port       : {red}{rest_port}{white}
* Slot duration   : {red}{slot_duration}{white}
* Slots per epoch : {red}{slots_per_epoch}{white}

########################################################

* CLI  version: {green}{jcli_version}{white}
* NODE version: {green}{jormungandr_version}{white}

########################################################

faucet_account: {green}{faucet_address}{white}
  * public: {blue}{faucet_public_key}{white}
  * secret: {red}{faucet_secret_key}{white}
  * amount: {green}{faucet_amount}{white}

pool_id: {green}{stake_pool_id}{white}

To start the node:
  {jormungandr} --genesis-block {genesis_path} --config {config_path} --secret {secret_path}

To connect using CLI REST:
  {jcli} rest v0 <CMD> --host {rest_url}
For example:
  {jcli} rest v0 node stats get --host {rest_url}
""".format(
        consensus=consensus,
        rest_port=args.rest_port,
        slot_duration=args.slot_duration,
        slots_per_epoch=args.slots_per_epoch,
        jcli_version=jcli_version,
        jormungandr_version=jormungandr_version,
        faucet_address=faucet_address,
        faucet_public_key=faucet_public_key,
        faucet_secret_key=faucet_secret_key,
        faucet_amount=INITIAL_FUNDS,
        stake_pool_id=stake_pool_id,
        jormungandr=jormungandr,
        genesis_path=block0_path,
        config_path=config_yaml_path,
        secret_path=pool_secret_path,
        jcli=jcli,
        rest_url=rest_listen,
        red=color_red,
        white=color_white,
        green=color_green,
        blue=color_blue,
    )
)
