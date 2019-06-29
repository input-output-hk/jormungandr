# Dockerfile

Dockerfile used to build or download, and run jormungandr.

## Instructions

The default build will download the latest release binaries.

### How To Build

```bash
docker build -t jormungandr-node:0.1 .
```

#### Build Options

The default build options are:
- directory: /app
- build mode: false
- jormungandr version: v0.2.3

These can be overridden during the docker build process:

To run a different version
```bash
docker build -t jormungandr-node:0.1 \
  --build-arg VERSION=v0.2.2 .
```

To build from source:
```bash
docker build -t jormungandr-node:0.1 \
  --build-arg BUILD=true .
```

To build a different version from source:
```bash
docker build -t jormungandr-node:0.1 \
  --build-arg BUILD=true \
  --build-arg VERSION=v0.2.2 .
```

### How to run

```bash
docker run jormungandr-node:0.1
```
