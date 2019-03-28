#! /bin/sh

cat << EOF
This test generate private key and public keys for every
supported algorithm
EOF

jcli='cargo run --bin jcli --'
key="${jcli} key"
generate="${key} generate"
to_public="${key} to-public"

try_algorithm() {
    algorithm=${1}

    ${generate} --type ${algorithm} | ${to_public}
    if [ ${?} -ne 0 ]; then
        cat >&2 << EOF
Test Failed for "${algorithm}"
EOF
        exit 1
    fi
}

try_algorithm ed25519
try_algorithm Ed25519Bip32
try_algorithm Ed25519Extended
try_algorithm Curve25519_2HashDH
try_algorithm FakeMMM

