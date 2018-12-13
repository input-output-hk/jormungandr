#!/usr/bin/env bash

# Jormaungandr setup tool - Amias Channer
#
# Generates a folder with a specified amount of interoperating jormungandr nodes
#



echo
echo "Jormungandr - Setup generator"
echo

clipath=$(which cardano-cli)


# params
# folder - folder to prepare for a demo
if [[ $1 = /* ]]; then
    folder="$1"
else
    folder="$PWD/${1#./}"
fi

# nodes - amount of nodes to prepare
nodes=${2:-0}

# genesis - genesis data for nodes to use
genesis=${3:-'demo-genesis.json'}

# cardano-cli - location of cardano-cli 
cli=${4:-$clipath}

# config - template for config
config=${5:-'demo-config.yaml'}

# IP - address to bind to
ip_address=${6:-'127.0.0.1'}

# port number to start from
minport=${7:-8000}

echo -e "Using these options:"
echo -e "\t Folder: $folder"
echo -e "\t Nodes: $nodes"
echo -e "\t Genesis: $genesis"
echo -e "\t CLI: $cli"
echo -e "\t Config: $config"
echo -e "\t IP: $ip_address"
echo -e "\t FirstPort: $minport"
echo

if [[ ! $folder ]]; then
  echo  "Error: Please supply a fully qualified folder name as the first parameter"
  exit 1
fi

if [[ -d $folder ]]; then
  echo  "Error: $folder already exists"
  exit 1
fi

if [[ $nodes == 0 ]]; then
  echo "Error: Please supply a node count as the second parameter"
  exit 1
fi

if [[ $genesis ]]; then
  if [[ ! -e $genesis ]]; then
    echo "Error: Cannot read genesis from $genesis"
    exit 1
  fi
else
  echo "Error: Please supply a genesis file as the third parameter"
  exit 1
fi

if [[ $cli ]]; then
  if [[ ! -e $cli ]]; then
    echo "Error: could not read cardano-cli at $cli"
    exit 1
  fi
else
  echo "Error: Please supply a cardano-cli executable as the fourth parameter"
  exit 1
fi

# we have all the info we need at this point

# exit if a command fails
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
  cd $node_folder

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

done  
popd > /dev/null

# copy keys but omitting the trailing comma , might not work pre bash4
trimmed_keys=${keys_for_genesis::-1}

echo "Copying in genesis and patching in keys"
cat $genesis  | jq -r '.bootStakeholders |= {'$trimmed_keys'}' > $folder/genesis.json
echo

# we loop again because we need all the keys in the config.yaml
# we will then apply the individual connection details to each nodes copy

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
echo 
  

