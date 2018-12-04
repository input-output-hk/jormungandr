#/bin/bash

# Stop Nodes - Amias Channer

# optional param - nodes to stop 
node_count=${1:-0}

nodes_found=`ls -1 ../nodes/ | wc -l`

if [ $node_count -gt $nodes_found ]; then
  echo "Cannot stop $node_count nodes , only $nodes_found are configured"
  exit
fi

if [ $node_count == 0 ]; then
  echo "Stopping all nodes"
  node_count=$nodes_found
fi

echo "Stopping $node_count Nodes"		
counter=0
while [ $counter -lt $node_count ]; do
			let counter=$counter+1		
	
			echo "Stopping Node $counter"
			stub='node_'$counter
			path='../nodes/'$counter'/'
			log=$stub'.log'
			cd $path
		
			screen -X -S $stub quit
			cd ../../bin		
done
