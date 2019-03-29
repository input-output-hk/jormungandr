#! /bin/sh

if [ "x${jcli}" = "x" ]; then
    SCRIPTPATH="$( cd "$(dirname "$0")" ; pwd -P )"
    source ${SCRIPTPATH}/../utils.sh
fi

title "check encode/decode between genesis file and block0"

info " running test ..."
command="${jcli} genesis init | ${jcli} genesis encode | ${jcli} genesis decode"
result=$(${jcli} genesis init | ${jcli} genesis encode | ${jcli} genesis decode)
if [ ${?} -ne 0 ]; then
    newline
    echo ${result} >&2
    warn "commands: ${command}\n"
    die " FAILED"
else
    success " PASSED"
    newline
fi
