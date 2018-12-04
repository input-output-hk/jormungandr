#/bin/bash

# Start Nodes - Amias Channer
#
# optional param - node count , amount of nodes to start

node_count=${1:-'0'}

nodes_found=`ls -1 ../nodes/ | wc -l`

if [ $node_count -gt $nodes_found ]; then
  echo "Cannot start $node_count nodes , only $nodes_found are configured"
  exit
fi

if [ $node_count == 0 ]; then
  echo "Starting all nodes"
  node_count=$nodes_found
fi

echo "Starting $node_count Nodes"		
counter=0
while [ $counter -lt $node_count ]; do
			let counter=$counter+1		
	
			echo "Starting Node $counter"
			stub='node_'$counter
			path='../nodes/'$counter'/'
			log=$stub'.log'
			cd $path
			
			screen -dmS $stub ../../bin/jormungandr --genesis-config ../../genesis.json --config ../../config.yaml --secret $stub.xprv  -vvv
			cd ../../bin		
done
