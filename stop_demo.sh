#!/bin/bash

node_count=${1:-'0'}

counter=0
while [ $counter -lt $node_count ]; do
	stub='node_'$counter
	screen -X -S $stub quit	
	let counter=$counter+1	
done
