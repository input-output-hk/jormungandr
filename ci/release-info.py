import json
import os
import re
import sys
from datetime import date
from subprocess import Popen, PIPE


def check_version(crate):
    # Checks package version for matching with the current tag reference
    if ref is not None and ref != "refs/tags/v" + str(crate[0]):
        return 0
    else:
        return 1


def print_error(crate, match):
    # Print errors for packages which versions didn't match tag reference
    if not match:
        print(
            "::error file={path}::version {version} does not match release tag {tag}".format(
                tag=ref, version=str(crate[0]), path=str(crate[1])
            )
        )


def bundle_version(crates):
    # Reads package versions from workspace manifest file
    channel = Popen(
        ["cargo", "metadata", "--format-version=1", "--no-deps"], stdout=PIPE
    )

    # parse json data
    data = json.load(channel.stdout).get("packages")

    # read, map and assign workspace crates versions to bundle package versions
    for package, _ in enumerate(data):
        if data[package]["name"] in crates:
            crates[data[package]["name"]].append(data[package]["version"])
            crates[data[package]["name"]].append(data[package]["manifest_path"])

    # Checks package versions of the crates bundle for consistency with the given tag reference
    consistency = list(map(check_version, list(crates.values())))

    # Print errors for packages which versions didn't match tag reference
    if not all(consistency):
        list(map(print_error, list(crates.values()), consistency))
        sys.exit(1)
    elif all(consistency):
        version = list(crates.values())[0][0]
        return version


event_name = sys.argv[1]

date = date.today().strftime("%Y%m%d")

ref = None
if event_name == "push":
    ref = os.getenv("GITHUB_REF")
    if ref.startswith("refs/tags/"):
        release_type = "tagged"
    elif ref == "refs/heads/ci/test/nightly":
        # emulate the nightly workflow
        release_type = "nightly"
        ref = None
    else:
        raise ValueError("unexpected ref " + ref)
elif event_name == "schedule":
    release_type = "nightly"
else:
    raise ValueError("unexpected event name " + event_name)


# Cargo workspace crates/packages for versioning bundle
crates = {
    "jormungandr": [],
    "jormungandr-lib": [],
    "jcli": [],
    "jormungandr-testing-utils": [],
    "jormungandr-integration-tests": [],
    "jormungandr-scenario-tests": [],
}

version = bundle_version(crates)
release_flags = ""
if release_type == "tagged":
    tag = "v" + version
elif release_type == "nightly":
    version = re.sub(
        r"^(\d+\.\d+\.\d+)(-.*)?$",
        r"\1-nightly." + date,
        version,
    )
    tag = "nightly." + date
    release_flags = "--prerelease"

for name in "version", "date", "tag", "release_type", "release_flags":
    print("::set-output name={0}::{1}".format(name, globals()[name]))
