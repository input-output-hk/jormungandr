#!/bin/bash

# ---------------------------------------------------------------
# Entrypoint script for voting-node container
# ---------------------------------------------------------------
#
# This script serves as the entrypoint for the jormungandr node.
#
# It expects the following environment variables to be set except where noted:
#
# STORAGE_PATH - The path where the node's data will be stored
# NODE_CONFIG_PATH - The path to the node's configuration file
# GENESIS_PATH (optional) - The path to the genesis block
# GENESIS_BUCKET (optional) - The S3 bucket where the genesis block is stored.
# GENESIS_ENV (optional) - The target environment. Used for fetching the genesis block from S3.
# GENESIS_FUND (optional) - The fund name. Used for fetching the genesis block from S3.
# GENESIS_VERSION (optional) - The genesis version. Used for fetching the genesis block from S3. If not set, the "default" version will be used.
# BFT_PATH (optional) - The path to the BFT file. Only used by leader nodes.
# LEADER (optional) - If set, the node will be configured as a leader node.
# DEBUG_SLEEP (optional) - If set, the script will sleep for the specified number of seconds before starting the node.
# ---------------------------------------------------------------

# Enable strict mode
set +x
set -o errexit
set -o pipefail
set -o nounset
set -o functrace
set -o errtrace
set -o monitor
set -o posix
shopt -s dotglob

check_env_vars() {
    local env_vars=("$@")

    # Iterate over the array and check if each variable is set
    for var in "${env_vars[@]}"; do
        echo "Checking $var"
        if [ -z "${!var+x}" ]; then
            echo ">>> Error: $var is required and not set."
            exit 1
        fi
    done
}

debug_sleep() {
    if [ -n "${DEBUG_SLEEP:-}" ]; then
        echo "DEBUG_SLEEP is set. Sleeping for ${DEBUG_SLEEP} seconds..."
        sleep "${DEBUG_SLEEP}"
    fi
}

echo ">>> Starting entrypoint script..."

REQUIRED_ENV=(
    "STORAGE_PATH"
    "NODE_CONFIG_PATH"
)
echo ">>> Checking required env vars..."
check_env_vars "${REQUIRED_ENV[@]}"

# Verify the storage path exists
if [[ ! -d "$STORAGE_PATH" ]]; then
    echo "ERROR: storage path does not exist at: $STORAGE_PATH"
    echo ">>> Aborting..."
    exit 1
fi

# Verify config is present
if [[ ! -f "$NODE_CONFIG_PATH" ]]; then
    echo "ERROR: node configuration is absent at: $NODE_CONFIG_PATH"
    echo ">>> Aborting..."
    exit 1
fi

# Verify genesis block is present or attempt to fetch from S3
if [[ -z "${GENESIS_PATH:=}" || ! -f "$GENESIS_PATH" ]]; then
    echo ">>> No genesis block provided. Attempting to fetch from S3..."

    REQUIRED_ENV=(
        "GENESIS_BUCKET"
        "GENESIS_ENV"
        "GENESIS_FUND"
    )
    echo ">>> Checking required env vars for fetching from S3..."
    check_env_vars "${REQUIRED_ENV[@]}"

    echo ">>> Fetching genesis block from S3 using the following parameters..."
    echo "Bucket: $GENESIS_BUCKET"
    echo "Environment: $GENESIS_ENV"
    echo "Fund: $GENESIS_FUND"
    echo "Version: ${GENESIS_VERSION:=}"

    GENESIS_PATH="$STORAGE_PATH/artifacts/block0.bin"
    mkdir -p "$(dirname "$GENESIS_PATH")"
    fetcher --bucket "$GENESIS_BUCKET" artifact -e "$GENESIS_ENV" -f "$GENESIS_FUND" -t "genesis" -v "${GENESIS_VERSION:=}" "$GENESIS_PATH"
fi

echo ">>> Using the following parameters:"
echo "Storage path: $STORAGE_PATH"
echo "Node config: $NODE_CONFIG_PATH"
echo "Genesis block: $GENESIS_PATH"

args+=()
args+=("--storage" "$STORAGE_PATH")
args+=("--config" "$NODE_CONFIG_PATH")
args+=("--genesis-block" "$GENESIS_PATH")

if [[ -n "${LEADER:=}" ]]; then
    echo ">>> Configuring node as leader..."

    # shellcheck disable=SC2153
    if [[ ! -f "$BFT_PATH" ]]; then
        echo "ERROR: BFT is absent at: $BFT_PATH"
        echo ">>> Aborting..."
        exit 1
    fi

    echo ">>> Using BFT at: $BFT_PATH"
    args+=("--secret" "$BFT_PATH")
fi

# Sleep if DEBUG_SLEEP is set
debug_sleep

echo "Starting node..."
exec "/app/jormungandr" "${args[@]}"
