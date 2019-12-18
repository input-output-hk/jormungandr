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

GITHUB_CHANGELOG_GENERATOR="$(which github_changelog_generator)"

if [ "x${GITHUB_CHANGELOG_GENERATOR}" = "x" ] || [ ! -x ${GITHUB_CHANGELOG_GENERATOR} ]; then
    echo "'github_changelog_generator' not installed? see https://github.com/github-changelog-generator/github-changelog-generator" >&2
    exit 1
fi

${GITHUB_CHANGELOG_GENERATOR} \
    --user input-output-hk \
    --project jormungandr \
    --output release.latest \
    --token ${GITHUB_TOKEN} \
    --breaking-labels "breaking-change" \
    --unreleased-only \
    --future-release ${NEW_VERSION}