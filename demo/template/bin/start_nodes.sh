#!/usr/bin/env bash

# Start Nodes - Amias Channer
#
# optional param - node min , lowest numberd node to start, defaults to 0
# optional param - node max , highest numberd node to start , defaults to all 

nodes_found=`ls -1 ../nodes/ | wc -l`

node_min=${1:-'1'}
node_max=${2:-$nodes_found}


if [[ $node_max -gt $nodes_found ]]; then
  echo "Cannot start $node_max nodes , only $nodes_found are configured"
  exit 1
fi

node_count=$((1 + $node_max - $node_min)) 

echo "Starting $node_count Nodes"		

for counter in $(seq $node_min $node_max)	
do
  echo "Starting Node $counter"

  stub='node_'$counter
  log=$stub'.log'
  path='../nodes/'$counter'/'
  # patch this when you have moved the config.yaml with topology in to the node folder
  command="../../bin/jormungandr --genesis-config genesis.json --config config.yaml --secret $stub.xprv --storage . -vvv "

  cd $path
  if [[ -e launch_cmd ]]; then
    echo "$stub is already running"
  else
    echo $command > launch_cmd
    echo "Running: $command"
    screen -dmS $stub -t $stub $command 2> $stub.log 1> $stub-error.log
  fi
  cd ../../bin
done
