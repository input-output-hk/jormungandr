#! /bin/bash

function usage {
    echo "${0} GITHUB_TOKEN NEW_VERSION" >&2
}

if [ ${#} -ne 2 ]; then
    usage
    exit 1
fi

if [ ${1} = "help" ]; then
    usage
    exit 0
fi

GITHUB_TOKEN=${1}
NEW_VERSION=${2}

# check that the command exists
command -v github_changelog_generator > /dev/null
# a non-zero code indicates that the command was not found
if [ $? -ne 0 ]; then
    echo "'github_changelog_generator' not installed? see https://github.com/github-changelog-generator/github-changelog-generator" >&2
    exit 1
fi


github_changelog_generator \
    --user input-output-hk \
    --project jormungandr \
    --output release.latest \
    --token ${GITHUB_TOKEN} \
    --breaking-labels "breaking-change" \
    --unreleased-only \
    --future-release ${NEW_VERSION}
