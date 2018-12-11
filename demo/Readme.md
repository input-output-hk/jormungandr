# Jormungandr Demo script

This code will allow you to setup a demo instance of jormnugndr with a configurable amount of nodes

## Prerequistes

* cardano-cli - a built version of cardano-cli will be used to generate keys
* genesis.json - a json file containing genesis data , look in cardano-deps/exe-common/genesis/

## Setup

First you need to create the environment to run jormungandr

`./setup.sh <folder> <nodecount> <genesis-file> <cardano-cli-path> <config.yaml> `

This will create the required environment in a folder, you only need to specify
the first 3 most of the time.

## Running 

### Starting nodes

once the folder is generated you can use the tools within it to start the nodes

`cd $folder/bin`

`./start_nodes.sh <min> <max>`

This will start the configured amount of nodes in individual screen sessions.
You can specify no parameters to start them all or a min and max to start all nodes in a range. 

### Stopping nodes

You can stop the nodes manually or with this script

`cd $folder/bin`

`./stop_nodes.sh <min> <max>`

This will stop the configured amount of nodes in individual screen sessions.
You can specify no parameters to stop them all or a min and max to stop all nodes in a range.


### Node Status

You can see available nodes by running

`cd $folder/bin`

`./list_nodes.sh`

This will show you a list of running nodes and their status.

## Advanced

All nodes are configured from a template stored in demo-template.yaml.
If you want to supply extra config options for the nodes edit this file
and it will be used by all nodes configured from this point onwards.
