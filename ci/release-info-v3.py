import json
import os
import re
import sys
from datetime import date
from subprocess import Popen, PIPE

def check_version(ver):
    # Checks package version for matching with the current tag reference
    # print(ver[0])
    if ref is not None and ref != 'refs/tags/v' + str(ver[0]):
        return 0
    else:
        return 1

def print_error(crate, ver, v):
    # print(crate, ver[0], v)
    if not v:
        print(
            '::error file={path}::version {0} does not match release tag {1}'
            .format(str(ver[0]), ref, path = str(crate) + '/Cargo.toml')
        )

def bundle_version(crates):
    # Reads package versions from workspace manifest file
    channel = Popen(
        ['cargo', 'metadata', '--format-version=1', '--no-deps'],
        stdout=PIPE
    )

    # parse json data
    data = json.load(channel.stdout).get('packages')

    # read, map and assign workspace crates versions to bundle package versions
    for package_id, _ in enumerate(data):
        if data[package_id]['name'] in crates:
            crates[data[package_id]['name']].append(data[package_id]['version'])
    #       print(package_id, data[package_id]['name'], data[package_id]['version'], crates)

    # Checks package versions of the crates bundle for consistency with the given tag reference
    consistency = list(map(check_version, list(crates.values())))
    # print(crates, consistency)

    # print errors for packages which versions didn't match tag reference
    if not all(consistency):
        list(map(print_error, list(crates.keys()), list(crates.values()), consistency))
        sys.exit(1)
    elif all(consistency):
        version = list(crates.values())[0][0]
    #   print(version)
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
crates = {
    'jormungandr':[],
    'jormungandr-lib':[],
    'jcli':[],
    'jormungandr-testing-utils':[],
    'jormungandr-integration-tests':[],
    'jormungandr-scenario-tests':[]
}

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

