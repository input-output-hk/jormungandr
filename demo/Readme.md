# Jormungandr Demo script

This code will allow you to setup a demo instance of jormnugndr with a configurable amount of nodes

## Prerequistes

* cardano-cli - a built version of cardano-cli will be used to generate keys
* genesis.json - a json file containing genesis data , look in cardano-deps/exe-common/genesis/

## Setup

First you need to create the environment to run jormungandr

`./setup.sh <folder> <nodecount> <genesis-file> <node-config> <cardano-cli-path>`

This will create the required environment in a folder.

## Running

### Starting nodes

once the folder is generated you can use the tools within it to start the nodes

`cd $folder/bin`

`./start_nodes.sh`


This will start the configured amount of nodes in individual screen sessions

### Stopping nodes

You can stop the nodes manually or with this script

`cd $folder/bin`

`./stop_nodes`


### Node Status

You can see available nodes by running

`cd $folder/bin`

`./list_nodes`


## Advanced

All nodes are configured from a template stored in demo-template.yaml.
If you want to supply extra config options for the nodes edit this file
and it will be used by all nodes configured from this point onwards.
