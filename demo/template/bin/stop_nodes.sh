#/bin/bash

# Stop Nodes - Amias Channer
#
# optional param - node min , lowest numberd node to Stop, defaults to 0
# optional param - node max , highest numberd node to Stop , defaults to all 

nodes_found=`ls -1 ../nodes/ | wc -l`

node_min=${1:-'1'}
node_max=${2:-$nodes_found}


if [ $node_max -gt $nodes_found ]; then
  echo "Cannot Stop $node_max nodes , only $nodes_found are configured"
  exit
fi

node_count=$((1 + $node_max - $node_min)) 

echo "Stoping $node_count Nodes"		

for counter in $(seq $node_min $node_max)	
do
	
			echo "Stopping Node $counter"
			stub='node_'$counter
			path='../nodes/'$counter'/'
			log=$stub'.log'
			cd $path
		  if [ -e launch_cmd ]; then
			  screen -X -S $stub quit
			else
			  echo "Doesn't seem to be running"
			fi
			cd ../../bin		
done
