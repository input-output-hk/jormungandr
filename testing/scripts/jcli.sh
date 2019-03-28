#! /bin/bash

SCRIPTPATH="$( cd "$(dirname "$0")" ; pwd -P )"
source ${SCRIPTPATH}/utils.sh

for test in $(ls ${SCRIPTPATH}/jcli)
do
    . ${SCRIPTPATH}/jcli/${test}
done
