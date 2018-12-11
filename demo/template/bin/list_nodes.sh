#!/usr/bin/env bash

# List nodes - Amias Channer


node_list=$(screen -ls | tail -n +2 | head -n -1 | cut -c 8-13 | sort)

node_count=$(echo -n $node_list | grep -c '^')

echo
echo "There are currently $node_count nodes running"
echo

for i in $node_list
do
  echo -n "Checking $i: "
  number=$(echo "$i" | sed 's/[^0-9]*//g')

  if [[ -e ../nodes/$number/launch_cmd ]]; then
    running=$(ps ax | grep $i | grep jormungandr | wc -l)
    if [[ $running -gt 0 ]]; then
      echo " Running"
    else
      echo " Not Running"
    fi
  else
    echo "Not Started"
  fi
done  

echo
echo "You can connect to a given node by running:"
echo "screen -r node_[1..$node_count]"
echo
echo "You can stop node 1 by running: "
echo "stop_nodes.sh 1 1"
echo
echo "You can start node 2 by running: "
echo "start_nodes.sh 2 2"
echo
