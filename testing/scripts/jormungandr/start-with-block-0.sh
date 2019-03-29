#! /bin/sh

if [ "${jcli}x" = "x" ]; then
    SCRIPTPATH="$( cd "$(dirname "$0")" ; pwd -P )"
    source ${SCRIPTPATH}/../utils.sh
fi

title "Check jormungandr can read the block-0"

info "  try starting node ..."
output=$(timeout 3s "${jormungandr} --config ${CONFIG} --genesis-block ${BLOCK0} --without-leadership")

echo ${output} | egrep --quiet 'panic|error'
if [ ${?} -eq 0 ]; then
    display ${output}
    newline
    die " FAILED"
else
    success " PASSED"
    newline
fi
