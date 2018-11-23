#!/bin/bash

node_count=${1:-'0'}
genesis=${2:-''}
config=${3:-'config.yaml'}

if [ ! -f $genesis ]; then
	echo "Could not find genesis file: "$genesis
else
	if [ $node_count == 0 ]; then
		echo "Usage: demo.sh <node_count> <genesis> <config>"
		exit
	else 
		echo "Making Configs"
		counter=0
		while [ $counter -lt $node_count ]; do
			let counter=$counter+1			
			
			stub='node_'$counter
			privkey=$stub'.xprv'
			pubkey=$stub'.xpub'

			echo "Making keys"
			./genkeypair.sh $privkey $pubkey

			echo "Adding key to config"
			pubkeycontents=`cat $pubkey` 
			echo "    - $pubkeycontents" >> $config
		done

		echo "Starting $node_count Nodes"		
		counter=0
		while [ $counter -lt $node_count ]; do
			let counter=$counter+1		
	
			echo "Starting Node $counter"
			stub='node_'$counter
			screen -dmS $stub cargo run  -- --genesis-config $genesis --config $config --secret $privkey  -vvv 			
		done
	fi
fi
