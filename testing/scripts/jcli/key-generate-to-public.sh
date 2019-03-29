#! /bin/sh

if [ "${jcli}x" = "x" ]; then
    SCRIPTPATH="$( cd "$(dirname "$0")" ; pwd -P )"
    source ${SCRIPTPATH}/../utils.sh
fi

title "Run generate private key and to public key"

try_algorithm() {
    algorithm=${1}

    info "  * ${algorithm} ..."
    command="${jcli} key generate --type ${algorithm} | ${jcli} key to-public"
    result=$(${jcli} key generate --type ${algorithm} | ${jcli} key to-public)
    if [ ${?} -ne 0 ]; then
        newline
        echo ${result} >&2
        warn "commands: ${command}\n"
        die " FAILED"
    else
        success " PASSED"
        newline
    fi
}

try_algorithm ed25519
try_algorithm Ed25519Bip32
try_algorithm Ed25519Extended
try_algorithm Curve25519_2HashDH
try_algorithm FakeMMM

