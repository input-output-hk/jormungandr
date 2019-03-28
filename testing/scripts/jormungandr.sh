#! /bin/bash

SCRIPTPATH="$( cd "$(dirname "$0")" ; pwd -P )"
source ${SCRIPTPATH}/utils.sh

info "Initial setup\n"

cat > ${CONFIG} << EOF
rest:
  listen: "127.0.0.1:8443"
  prefix: "api"

logger:
  verbosity: 7
  format: json
peer_2_peer:
  trusted_peers: []
  public_access: "/ip4/127.0.0.1/tcp/8081"
  topics_of_interests:
    transactions: low
    blocks: normal
EOF

${jcli} genesis init | ${jcli} genesis encode --output ${BLOCK0}

for test in $(ls ${SCRIPTPATH}/jormungandr)
do
    . ${SCRIPTPATH}/jormungandr/${test}
done

