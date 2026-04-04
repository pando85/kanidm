# This builds the kanidm CLI tools
ARG BASE_IMAGE=debian:trixie-slim

FROM ${BASE_IMAGE} AS builder
ARG KANIDM_FEATURES
ARG KANIDM_BUILD_PROFILE
ARG KANIDM_BUILD_OPTIONS=""

ENV KANIDM_BUILD_PROFILE=${KANIDM_BUILD_PROFILE:-container_generic}
ENV RUSTFLAGS="-Clinker=clang"

RUN apt-get update && apt-get install -y --no-install-recommends \
    sccache \
    cargo \
    clang \
    gcc \
    make \
    automake \
    autoconf \
    libpam0g-dev \
    libudev-dev \
    libsqlite3-dev \
    rsync \
    && rm -rf /var/lib/apt/lists/*

COPY . /usr/src/kanidm

WORKDIR /usr/src/kanidm/

RUN \
    --mount=type=cache,id=cargo,target=/cargo \
    --mount=type=cache,id=sccache,target=/sccache \
    export CARGO_HOME=/cargo; \
    export SCCACHE_DIR=/sccache; \
    export RUSTC_WRAPPER=/usr/bin/sccache; \
    export CC="/usr/bin/clang"; \
    cargo build --locked -p orca ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kanidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo install \
        --git https://github.com/kanidm/webauthn-rs.git \
        --rev 5f4db4172f8e22aedc68c282d177e98db2b1892f \
        --force fido-mds-tool \
        --target-dir="/usr/src/kanidm/target/" && \
    sccache -s

FROM ${BASE_IMAGE}

ENV RUST_BACKTRACE=1

COPY --from=builder /usr/src/kanidm/target/release/orca /sbin/
COPY ./tools/orca/profile-sample.toml /etc/kanidm/profile-sample.toml

RUN chmod +x /sbin/orca

CMD [ "/sbin/orca", "-h" ]
