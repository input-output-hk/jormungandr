import json
import os
import re
import sys
from datetime import date
from subprocess import Popen, PIPE

def read_version(crate):
    # Reads package version from workspace manifest file
    channel = Popen(
        ['cargo', 'read-manifest', '--manifest-path', './{}/Cargo.toml'.format(crate)],
        stdout=PIPE
    )

    # parse json data
    data = json.load(channel.stdout)
    version = data['version']
    return version

def check_version(ver):
    # Checks package version for matching with the current tag reference
    if ref is not None and ref != 'refs/tags/v' + str(ver):
        return 0
    else:
        return 1

def print_error(crate, ver, match):
    # print(crate, ver, match)
    if not match:
        print(
            '::error file={path}::version {version} does not match release tag {tag}'
            .format(tag = ref, version = str(ver), path = str(crate) + '/Cargo.toml')
        )

def bundle_version(crates):
    # Checks package versions of the crates bundle for consistency with the given tag reference
    bundle = list(map(read_version, crates))
    consistency = list(map(check_version, bundle))
    # print(crates, bundle, consistency)

    # print errors for packages which versions didn't match tag reference
    if not all(consistency):
        list(map(print_error, crates, bundle, consistency))
        sys.exit(1)
    elif all(consistency):
        version = bundle[0]
        return version


event_name = sys.argv[1]

date = date.today().strftime('%Y%m%d')

ref = None
if event_name == 'push':
    ref = os.getenv('GITHUB_REF')
    if ref.startswith('refs/tags/'):
        release_type = 'tagged'
    elif ref == 'refs/heads/ci/test/nightly':
        # emulate the nightly workflow
        release_type = 'nightly'
        ref = None
    else:
        raise ValueError('unexpected ref ' + ref)
elif event_name == 'schedule':
    release_type = 'nightly'
else:
    raise ValueError('unexpected event name ' + event_name)


# Cargo workspace crates/packages for versioning bundle
crates = [
    'jormungandr',
    'jormungandr-lib',
    'jcli',
    'testing/jormungandr-testing-utils',
    'testing/jormungandr-integration-tests',
    'testing/jormungandr-scenario-tests'
]

version = bundle_version(crates)
release_flags = ''
if release_type == 'tagged':
    tag = 'v' + version
elif release_type == 'nightly':
    version = re.sub(
        r'^(\d+\.\d+\.\d+)(-.*)?$',
        r'\1-nightly.' + date,
        version,
    )
    tag = 'nightly.' + date
    release_flags = '--prerelease'

for name in 'version', 'date', 'tag', 'release_type', 'release_flags':
    print('::set-output name={0}::{1}'.format(name, globals()[name]))

