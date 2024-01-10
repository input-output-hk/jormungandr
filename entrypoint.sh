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
# ENV (optional) - The target environment. Used for fetching the node's configuration and archives from S3.
# GENESIS_PATH (optional) - The path to the genesis block
# GENESIS_BUCKET (optional) - The S3 bucket where the genesis block is stored.
# GENESIS_FUND (optional) - The fund name. Used for fetching the genesis block from S3.
# GENESIS_VERSION (optional) - The genesis version. Used for fetching the genesis block from S3. If not set, the "default" version will be used.
# ARCHIVE_BUCKET (optional) - The S3 bucket where archives are stored.
# ARCHIVE_ID (optional) - If present, the node will attempt to fetch the archive with the specified ID from S3 and restore it.
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

fetch_archive() {
    local bucket=$1
    local env=$2
    local id=$3
    local archive_path=$4

    echo ">>> Fetching archive from S3 using the following parameters..."
    echo "Bucket: $bucket"
    echo "Environment: $env"
    echo "Archive ID: $id"

    fetcher --bucket "$bucket" archive -e "$env" -i "$id" "$archive_path"

    echo ">>> Clearing storage path..."
    rm -rf "${STORAGE_PATH:?}/*"

    echo ">>> Extracting archive..."
    zstd -cd "$archive_path" | tar xf - -C "$STORAGE_PATH"
    rm "$archive_path"

    echo ">>> Setting last restored archive to $ARCHIVE_ID"
    echo "$id" >"$STORAGE_PATH/last_restored_archive"
}

fetch_genesis() {
    local bucket=$1
    local env=$2
    local fund=$3
    local version=$4
    local path=$5

    echo ">>> Fetching genesis block from S3 using the following parameters..."
    echo "Bucket: $bucket"
    echo "Environment: $env"
    echo "Fund: $fund"
    echo "Version: ${version}"

    mkdir -p "$(dirname "$GENESIS_PATH")"
    fetcher --bucket "$bucket" artifact -e "$env" -f "$fund" -t "genesis" -v "${version}" "$path"
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
        "ENV"
        "GENESIS_BUCKET"
        "GENESIS_FUND"
    )
    echo ">>> Checking required env vars for fetching from S3..."
    check_env_vars "${REQUIRED_ENV[@]}"

    GENESIS_PATH="$STORAGE_PATH/artifacts/block0.bin"
    fetch_genesis "$GENESIS_BUCKET" "$ENV" "$GENESIS_FUND" "${GENESIS_VERSION:=}" "$GENESIS_PATH"
fi

# Check if we need to pull an archive from S3
if [[ -n "${ARCHIVE_ID:=}" ]]; then
    echo ">>> Archive ID provided. Attempting to fetch from S3..."
    ARCHIVE_PATH="/tmp/archive.tar.zstd"

    REQUIRED_ENV=(
        "ENV"
        "ARCHIVE_BUCKET"
        "ARCHIVE_ID"
    )
    echo ">>> Checking required env vars for fetching the archive from S3..."
    check_env_vars "${REQUIRED_ENV[@]}"

    echo ">>> Checking if the archive has already been restored..."
    if [[ -f "$STORAGE_PATH/last_restored_archive" ]]; then

        LAST_RESTORED_ARCHIVE=$(cat "$STORAGE_PATH/last_restored_archive")

        if [[ "$LAST_RESTORED_ARCHIVE" == "$ARCHIVE_ID" ]]; then
            echo ">>> Archive $ARCHIVE_ID has already been restored. Skipping..."
        else
            fetch_archive "$ARCHIVE_BUCKET" "$ENV" "$ARCHIVE_ID" "$ARCHIVE_PATH"
        fi
    else
        fetch_archive "$ARCHIVE_BUCKET" "$ENV" "$ARCHIVE_ID" "$ARCHIVE_PATH"
    fi
fi

echo ">>> Using the following parameters:"
echo "Storage path: $STORAGE_PATH"
echo "Node config: $NODE_CONFIG_PATH"
echo "Genesis block: $GENESIS_PATH"
echo "Genesis block hash (SHA256): $(sha256sum "$GENESIS_PATH" | awk '{ print $1 }')"

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
