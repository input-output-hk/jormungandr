#!/usr/bin/env bash

# Jormaungandr setup tool - Amias Channer
#
# Generates a folder with a specified amount of interoperating jormungandr nodes
#
echo
echo "Jormungandr - Setup generator"
echo

clipath=$(which cardano-cli)

TEMP=`getopt -o -n:g:c:f: --long nodes:,config::,genesis:,flavour: -n 'setup.sh' -- "$@"`
eval set -- "$TEMP"

# nodes - amount of nodes to prepare
nodes=0;
# genesis - genesis data for nodes to use
genesis="demo-genesis.json"
# config - template for config
config="demo-config.yaml"
# build flavour
flavour="local:127.0.0.1:8000"

while true; do
  case $1 in
    -n|--nodes) nodes=$2;shift 2 ;;
    -g|--genenis) genesis=$2;shift 2 ;;
    -c|--config) config=$2; shift 2 ;;
    -f|--flavour) flavour=$2; shift 2 ;;
    --) shift ; break ;;
    *) folder=$1; shift 1; ;;
  esac
done


OFS=$IFS
IFS=":"
eval set -- $flavour
if [ x$1 = xlocal ] ; then
  ip_address=$2
  minport=$3
  flavourdesc="Local setup { ip: $ip_address, minport: $minport }"
elif [ x$1 = xdocker ] ; then
  flavourdesc="Docker setup"
else
  echo "unknown flavour: $1"
  exit 1
fi
IFS=$OFS

# params
# folder - folder to prepare for a demo
if [[ $folder = /* ]]; then
    folder="$folder"
else
    folder="$PWD/${folder#./}"
fi

# cardano-cli - location of cardano-cli 
cli=$clipath

cat << OPTIONS
Using these options:
	Folder: $folder
	Nodes: $nodes
	Genesis: $genesis
	CLI: $cli
	Config: $config
        Flavour: $flavourdesc
OPTIONS

if [[ ! $folder ]]; then
  echo  "Error: Please supply a fully qualified folder name as the first parameter"
  exit 1
fi
#
if [[ -d $folder ]]; then
  echo  "Error: $folder already exists"
  exit 1
fi

if [[ $nodes == 0 ]]; then
  echo "Error: Please supply a node count using -n or --nodes"
  exit 1
fi

if [[ $genesis ]]; then
  if [[ ! -e $genesis ]]; then
    echo "Error: Cannot read genesis from $genesis"
    exit 1
  fi
else
  echo "Error: Please supply a genesis file using -g or --genesis"
  exit 1
fi

if [[ $cli ]]; then
  if [[ ! -e $cli ]]; then
    echo "Error: could not read cardano-cli at $cli"
    exit 1
  fi
else
  echo "Error: Please supply a cardano-cli in environment"
  exit 1
fi

## we have all the info we need at this point
#
## exit if a command fails
set +e

echo "Building Jormungandr"
echo
cd ..
cargo build
cd demo
echo
echo "Build finished"
echo

mkdir -p $folder  

echo "Copying in template"
cp -rv template/* $folder/
echo

echo "Copying in binaries"
cp $cli $folder/bin/cardano-cli
cp ../target/debug/jormungandr $folder/bin/
echo


echo "Making Configs for $nodes nodes"
echo
counter=0
keys_for_genesis=''

cp $config $folder'/config.yaml'

pushd . > /dev/null
for (( counter=1; counter<=$nodes; counter+=1 )); do
  stub='node_'$counter
  node_folder=$folder/nodes/$(($counter))

  echo -e "\t Making $stub folder"
  mkdir -p $node_folder
  chmod a+rwx $node_folder
  pushd $node_folder > /dev/null

  privkey=$node_folder/$stub'.xprv'
  pubkey=$node_folder/$stub'.xpub'

  echo -e "\t Making keys for $stub"
  echo -e "\t\t PRIV = $privkey"
  echo -e "\t\t PUB  = $pubkey"

  ../../bin/cardano-cli debug generate-xprv $privkey
  ../../bin/cardano-cli debug xprv-to-xpub $privkey $pubkey

  echo -e "\t Adding $stub public key to global config"
  pubkeycontents=`cat $pubkey`
  echo "    - $pubkeycontents" >> $folder'/config.yaml'
    
  echo -e "\t Adding $stub private key to genesis file"
  privkeycontents=`cat $privkey`
  keys_for_genesis+='"'$privkeycontents'":1,'
  echo
  popd > /dev/null

done  
popd > /dev/null

trimmed_keys=${keys_for_genesis::-1}

echo "Copying in genesis and patching in keys"
cat $genesis  | jq -r '.bootStakeholders |= {'$trimmed_keys'}' > $folder/genesis.json
echo
 

# copy keys but omitting the trailing comma , might not work pre bash4

# we loop again because we need all the keys in the config.yaml
# we will then apply the individual connection details to each nodes copy
if [ $flavour == "local" ]; then
echo "Applying Topology"
lastport=$minport
counter=1
echo
pushd . > /dev/null
for (( counter=1; counter<=$nodes; counter+=1 )); do
  stub='node_'$counter
  node_folder=$folder/nodes/$(($counter))

  echo -e "\t Configuring $stub"
  cd $node_folder

  echo -e "\t\t Generating port numbers"
  listen_port=$((lastport+1))
  connect_port=$((listen_port+1))

  # connect the last to the first
  if [[ $counter -eq $nodes ]]; then
    connect_port=$((minport+1))
  fi

  echo -e "\t\t\t Listen: $listen_port"
  echo -e "\t\t\t Connect: $connect_port"


  echo -e "\t\t Patching in Yaml Config"
  cp $folder/config.yaml $node_folder/config.yaml

  cat << END >> $node_folder/config.yaml
legacy_listen:
  - "$ip_address:$listen_port"
legacy_connect:
  - "$ip_address:$connect_port"
END

  echo -e "\t\t Copying in Genesis"
  cp $folder/genesis.json $node_folder/genesis.json

  echo
  # move forward one step
  lastport=listen_port
done
popd > /dev/null
echo


echo "Setup is complete"
echo 
echo "Its not Ragnarok yet but if you want to unleash your jormungandr do this:"
echo
echo "cd $folder/bin/" 
echo "./start_nodes.sh"

elif [ $flavour = "docker" ] ; then

compose=$folder/docker-compose.yaml

docker_nodes=""
for (( counter=1; counter<=$nodes; counter+=1 )); do
   docker_nodes="${docker_nodes}    - node${counter}:10000
"
done


nodeslist=""
for ((i=1; $i <= $nodes; i=$i+1)) ; do
  nodeslist="${nodeslist} node${i}"
done

# Update config
for ((i=1; $i <= $nodes; i=$i+1)); do
   remote_nodes=`sed /node${i}/d <<< $docker_nodes`
   node_folder="$folder"/nodes/"${i}"
   echo "Copying node-${i} genesis"
   cp $folder/genesis.json $node_folder/genesis.json
   echo "Copying node-${i} config"
   cp $folder/config.yaml $node_folder/config.yaml
   cat >> $node_folder/config.yaml << END
legacy_listen:
  - node${i}:10000
legacy_peers:   
$remote_nodes
END
done

cat > ${compose} << END
# Docker compose file for the demo jormungandr networks
version: '3'

services:
END

for ((i=1; $i<=$nodes; i=$i+1)); do

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
for ((i=1; $i <= $nodes; i=$i+1)); do
cat >> ${compose} << END
    node${i}_volume:
END
done

cat >> ${compose} << END
networks:
   private:
END

echo 
echo "If you prefer to use docker containers, then:"
echo "cd $folder"
echo "./bin/gen_compose.sh"
echo
echo " And follow instructions"
fi
