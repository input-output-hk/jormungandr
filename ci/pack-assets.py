# arguments: jormungandr version, target triple, target cpu

import sys
import platform
import hashlib
import shutil
import os


def sha256sum(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        data = f.read()
        h.update(data)
    return h.hexdigest()


version = sys.argv[1]
target = sys.argv[2]
target_cpu = sys.argv[3]

archive_basename = "jormungandr-{}-{}-{}".format(version, target, target_cpu)

root_dir = "./target/{}/release".format(target)

if platform.system() == "Windows":
    jormungandr_name = "jormungandr.exe"
    jcli_name = "jcli.exe"
else:
    jormungandr_name = "jormungandr"
    jcli_name = "jcli"

jormungandr_path = os.path.join(root_dir, jormungandr_name)
jcli_path = os.path.join(root_dir, jcli_name)

jormungandr_checksum = sha256sum(jormungandr_path)
jcli_checksum = sha256sum(jcli_path)

# build archive
if platform.system() == "Windows":
    import zipfile

    content_type = "application/zip"
    archive_name = "{}.zip".format(archive_basename)
    with zipfile.ZipFile(archive_name, mode="x") as archive:
        archive.write(jormungandr_path, arcname=jormungandr_name)
        archive.write(jcli_path, arcname=jcli_name)
else:
    import tarfile

    content_type = "application/gzip"
    archive_name = "{}.tar.gz".format(archive_basename)
    with tarfile.open(archive_name, "x:gz") as archive:
        archive.add(jormungandr_path, arcname=jormungandr_name)
        archive.add(jcli_path, arcname=jcli_name)

# verify archive
shutil.unpack_archive(archive_name, "./unpack-test")
jormungandr1_checksum = sha256sum(os.path.join("./unpack-test", jormungandr_name))
jcli1_checksum = sha256sum(os.path.join("./unpack-test", jcli_name))
shutil.rmtree("./unpack-test")
if jormungandr1_checksum != jormungandr_checksum:
    print(
        "jormungandr checksum mismarch: before {} != after {}".format(
            jormungandr_checksum, jormungandr1_checksum
        )
    )
    exit(1)
if jcli1_checksum != jcli_checksum:
    print(
        "jcli checksum mismarch: before {} != after {}".format(
            jcli_checksum, jcli1_checksum
        )
    )
    exit(1)

# save archive checksum
archive_checksum = sha256sum(archive_name)
checksum_filename = "{}.sha256".format(archive_name)
with open(checksum_filename, "x") as f:
    f.write(archive_checksum)

# set GitHub Action step outputs
print("::set-output name=release-archive::{}".format(archive_name))
print("::set-output name=release-content-type::{}".format(content_type))
