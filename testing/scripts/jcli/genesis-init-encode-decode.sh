#! /bin/sh

cat << EOF
This test that the `genesis init` command it working as expected:

* that it is a valid genesis file (it encodes into a block 0);
* that the block0 generated is a valid block 0 (it decodes to genesis file)
EOF

jcli='cargo run --bin jcli --'

${jcli} genesis init | \
    ${jcli} genesis encode | \
    ${jcli} genesis decode

if [ ${?} -ne 0 ]; then
    cat >&2 << EOF
Test Failed
EOF
    exit 1
fi
