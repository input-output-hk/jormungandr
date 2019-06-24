# Simple dockerfile example to build a jormungandr and start in genesis mode

FROM alpine:3.9.4

LABEL MAINTAINER IOHK
LABEL description="Jormungandr"

ARG VERSION=0.2.3
ARG PREFIX=/app
ARG REST_PORT=8448

ENV ENV_PREFIX=${PREFIX}

# prepare the environment
RUN apk add --no-cache curl

# get jormungandr and jcli from repo releases
RUN mkdir -p ${ENV_PREFIX}/bin && \
    cd ${ENV_PREFIX}/bin && \
    curl -L https://github.com/input-output-hk/jormungandr/releases/download/v${VERSION}/jormungandr-v${VERSION}-x86_64-unknown-linux-musl.tar.gz | tar xz

# get scripts from repo archive
RUN mkdir ${ENV_PREFIX}/src && \
    cd ${ENV_PREFIX}/src && \
    curl -L https://github.com/input-output-hk/jormungandr/archive/v${VERSION}.tar.gz | tar xz && \
    cp jormungandr-${VERSION}/scripts/* ${ENV_PREFIX}/bin/ && \
    rm -r ${ENV_PREFIX}/src

ENV PATH=${ENV_PREFIX}/bin:${PATH}

WORKDIR ${ENV_PREFIX}/bin

RUN mkdir -p ${ENV_PREFIX}/cfg && \
    mkdir -p ${ENV_PREFIX}/secret && \
    sh ./bootstrap -p ${REST_PORT} -x -c ${ENV_PREFIX}/cfg -k ${ENV_PREFIX}/secret

EXPOSE ${REST_PORT}

CMD [ "sh", "startup_script.sh" ]
