#/bin/bash -e

# Jormaungandr setup tool - Amias Channer
#
# Generates a folder with a specified amount of interoperating jormungandr nodes
#
echo
echo "Jormunandr - Setup generator"
echo

# params
# folder - folder to prepare for a demo
folder=$1

# nodes - amount of nodes to prepare
nodes=${2:-0}

# genesis - genesis data for nodes to use
genesis=${3:-0}

# config - template for config
config=${4:-'demo-config.yaml'}

# cardano-cli - location of cardano-cli 
cli=${5:-"cardano-cli"}


echo "Using these options:"
echo "  Folder: $folder"
echo "  Nodes: $nodes"
echo "  Genesis: $genesis"
echo "  Config: $config"
echo "  CLI: $cli"
echo

if [ ! $folder ]; then
  echo  "Error: Please supply a fully qualified folder name as the first parameter"
  exit
fi

if [ -d $folder ]; then
  echo  "Error: $folder already exists"
  exit
fi

if [ $nodes == 0 ]; then
  echo "Error: Please supply a node count as the second parameter"
  exit
fi

if [ $genesis ]; then
  if [ ! -e $genesis ]; then
    echo "Error: Cannot read genesis from $genesis"
    exit
  fi
else
  echo "Error: Please supply a genesis file as the third parameter"
  exit
fi

# we have all the info we need at this point

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
cp $cli $folder/bin/
cp ../target/debug/jormungandr $folder/bin/
echo

echo "Copying in genesis"
cp $genesis $folder/genesis.json
echo

echo "Making Configs for $nodes nodes"
counter=0
cp $config $folder'/config.yaml'

while [ $counter -lt $nodes ]; do
    
    node_folder=$folder/nodes/$(($counter+1))
    mkdir -p $node_folder
    cd $node_folder
    
    let counter=$counter+1
    stub='node_'$counter
    privkey=$stub'.xprv'
    pubkey=$stub'.xpub'

    echo "Making keys for $stub"
    echo "PRIV = $privkey"
    echo "PUB  = $pubkey"

    echo "$cli debug generate-xprv $privkey"
    $cli debug generate-xprv $privkey
    echo "$cli debug xprv-to-xpub $privkey $pubkey"
    $cli debug xprv-to-xpub $privkey $pubkey

    echo "Adding key to global config"
    pubkeycontents=`cat $pubkey` 
    echo "    - $pubkeycontents" >> '../../config.yaml'
    echo
done  

echo "Setup is complete"
echo 
echo "Its not Ragnarok yet but if you want to unleash your jormungandr do this:"
echo
echo "cd $folder/bin/" 
echo "./start_nodes.sh"
echo 
  

