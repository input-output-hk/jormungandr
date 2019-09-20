#! /bin/sh

PACKAGES='jormungandr jormungandr-lib jcli jormungandr-integration-tests jormungandr-scenario-tests'

if [ "${EDITOR}x" = "x" ]; then
    echo "> no environment variable \`EDITOR', trying known editors'"

    for CANDIDATE in vim emacs nano
    do
        echo ">> trying \`${CANDIDATE}'"
        which ${CANDIDATE} &> /dev/null
        if [ ${?} -eq 0 ]; then
            echo ">> \`${CANDIDATE}' found, using it as an editor"
            export EDITOR=${CANDIDATE}
            break;
        fi
    done
fi

if [ "${EDITOR}x" = "x" ]; then
    echo "> no known editor... giving up"
    exit 1
fi

for PACKAGE in ${PACKAGES}
do
    ${EDITOR} ${PACKAGE}/Cargo.toml
done