#!/bin/sh
patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 bin/jormungandr
