#!/bin/bash

node_count=${1:-'0'}
genesis=${2:-''}
config=${3:-'config.yaml'}

if [ ! -f $genesis ]; then
	echo "Could not find genesis file: "$genesis
else
	if [ $node_count == 0 ]; then
		echo "Usage: demo.sh <node_count> <genesis>"
		exit
	else 
		counter=0
		while [ $counter -lt $node_count ]; do
			stub='node_'$counter
			privkey=$stub'.xprv'
			pubkey=$stub'.xpub'
			echo "Making keys"
			./genkeypair.sh $privkey $pubkey
			echo "Adding key to config"
			pubkeycontents=`cat $pubkey` 
			echo "Key = $pubkeycontents"
			echo "    - $pubkeycontents" >> $config
			echo "Starting Node with keys"
			screen -dmS $stub cargo run  -- --genesis-config $genesis --config $config --secret $privkey  -vvv 
			let counter=$counter+1
		done

	fi
fi
