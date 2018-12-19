#!/bin/bash

ROOT=./

cat << WELCOME
Generating docker-compose file for running demo.
After running a script new docker-compose.yaml
file will be generated in your demo directory.

You can run this file using \`docker-compose up --build\`
command.  This will run the private network with nodes
running in it and create all persisted volumes.

Jormungandr executable may be updated by placing it to the
\`bin/jormungandr\` and calling \`docker-compose up --build\` again.
All volumes will be persisted in that case.
Config files may be updated in the \`nodes/\$n/config.yaml\` directly.
In order to clean volumes one must run \`docker volumes prune\` explicitly.

N.B. NixOS users must run \`./bin/fix-nix.sh\` script before building a
container otherwise \`jormungandr\` executable will refuse to work)
WELCOME

n=`ls -1 ${ROOT}/nodes/ | wc -l`
compose=${ROOT}/docker-compose.yaml

nodes=""

for ((i=1; $i<=$n; i=$i+1)) ; do
  nodes="${nodes}    - node${i}:10000
"
done

nodeslist=""
for ((i=1; $i<=$n; i=$i+1)) ; do
  nodeslist="${nodeslist} node${i}"
done

for ((i=1; $i<=$n; i=$i+1)); do
   cp ${ROOT}/config.yaml ${ROOT}/nodes/"${i}"/
   remote_nodes=`sed /node${i}/d <<< $nodes`
   cat >> ${ROOT}/nodes/"${i}"/config.yaml << END
legacy_listen:
  - node${i}:10000
legacy_peers:   
$remote_nodes
END

done;

cat > ${compose} << END
# Docker compose file for the demo jormungandr networks
version: '3'

services:
END


for ((i=1; $i<=$n; i=$i+1)); do

cat >> ${compose} << END
    node${i}:
       build: ./
       image: "jormungandr"
       networks:
         - private
       volumes:
         - node${i}_volume:/var/lib/cardano/data
         - ./genesis.json:/etc/jormungandr/genesis.json
         - ./nodes/${i}/:/var/lib/cardano/etc/
       environment:
         - RUST_BACKTRACE=1
       command:
         - /var/lib/cardano/bin/jormungandr_wrapper
         - ${nodeslist}
         - /var/lib/cardano/etc/config.yaml
         - /var/lib/cardano/bin/jormungandr
         - --config
         - /var/lib/cardano/etc/config.yaml.tmp
         - --genesis-config
         - /etc/jormungandr/genesis.json
         - --secret
         - /var/lib/cardano/etc/node_${i}.xprv
         - --storage
         - /var/lib/cardano/data
         - -vvv
END

done;

cat >> ${compose} << END
volumes:
END
for ((i=1; $i<=$n; i=$i+1)); do
cat >> ${compose} << END
    node${i}_volume:
END
done

cat >> ${compose} << END
networks:
   private:
END
