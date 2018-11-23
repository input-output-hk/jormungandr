#!/bin/bash

node_count=${1:-'0'}

counter=0
while [ $counter -lt $node_count ]; do
	let counter=$counter+1	
	
	stub='node_'$counter
	screen -X -S $stub quit			
done
