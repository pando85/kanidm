# Build the main Kanidmd server
ARG BASE_IMAGE=debian:trixie-slim

FROM ${BASE_IMAGE} AS builder
ARG KANIDM_FEATURES
ARG KANIDM_BUILD_PROFILE="container_generic"
ARG KANIDM_BUILD_OPTIONS=""

ENV KANIDM_BUILD_PROFILE=${KANIDM_BUILD_PROFILE:-container_generic}
ENV RUSTFLAGS="-Clinker=clang"

RUN apt-get update && apt-get install -y --no-install-recommends \
    sccache \
    cargo \
    cargo-auditable \
    clang \
    gcc \
    cmake \
    gawk \
    make \
    automake \
    autoconf \
    libpam0g-dev \
    libsqlite3-dev \
    libsystemd-dev \
    rsync \
    findutils \
    && rm -rf /var/lib/apt/lists/*

COPY . /usr/src/kanidm

WORKDIR /usr/src/kanidm/server/daemon

RUN --mount=type=cache,id=cargo,target=/cargo \
    --mount=type=cache,id=sccache,target=/sccache \
    export CARGO_HOME=/cargo && \
    export SCCACHE_DIR=/sccache && \
    export RUSTC_WRAPPER=/usr/bin/sccache && \
    export CC="/usr/bin/clang" && \
    cargo auditable build --locked -p daemon ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kanidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release; \
    sccache -s

RUN <<EOF
    mkdir -p /out/libs
    mkdir -p /out/libs-root
    ldd /usr/src/kanidm/target/release/kanidmd
    ldd /usr/src/kanidm/target/release/kanidmd | grep -v 'linux-vdso.so' | awk '{print $(NF-1) " " $1}' | sort -u -k 1,1 | awk '{print "install", "-D", $1, (($2 ~ /^\//) ? "/out/libs-root" $2 : "/out/libs/" $2)}' | xargs -I {} sh -c {}
    ls -Rla /out/libs
    ls -Rla /out/libs-root
EOF

FROM scratch

WORKDIR /

COPY --from=builder /etc/ssl/certs /etc/ssl/certs

COPY --from=builder --chmod=0755 /usr/src/kanidm/target/release/kanidmd /sbin/
COPY --from=builder /usr/src/kanidm/server/core/static /hpkg/

COPY --from=builder /out/libs-root/ /
COPY --from=builder /out/libs/ /lib/

ENV LD_LIBRARY_PATH=/lib

WORKDIR /data

EXPOSE 8443 3636

ENV RUST_BACKTRACE=1

HEALTHCHECK \
    --interval=60s \
    --timeout=10s \
    --start-period=60s \
    --start-interval=5s \
    --retries=3 \
    CMD [ "/sbin/kanidmd", "scripting", "healthcheck"]

CMD [ "/sbin/kanidmd", "server"]
