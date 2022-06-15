#!/bin/sh


if [ $# -ne 2 ]; then
    echo "usage: $0 <ENDPOINT> <OUTPUT_SCHEMA>"
    echo "    <ENDPOINT>  graphql endpoint (http://127.0.0.1:10003/graphql)"
    echo "    <OUTPUT_SCHEMA> schema output location (schema.graphql)"
    exit 1
fi

echo "================Explorer schema generator ================="
echo "This Script will dump schema into: '$2'"

echo $(npm install -g get-graphql-schema)
echo $(get-graphql-schema $1 > $2)
echo "================ DONE ====================================="
