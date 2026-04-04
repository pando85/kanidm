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
    cargo-auditable \
    clang \
    gcc \
    make \
    automake \
    autoconf \
    libpam0g-dev \
    libudev-dev \
    libsqlite3-dev \
    ca-certificates \
    rsync \
    && rm -rf /var/lib/apt/lists/*

ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

COPY . /usr/src/kanidm

WORKDIR /usr/src/kanidm/

RUN \
    --mount=type=cache,id=cargo,target=/cargo \
    --mount=type=cache,id=sccache,target=/sccache \
    export CARGO_HOME=/cargo; \
    export SCCACHE_DIR=/sccache; \
    export RUSTC_WRAPPER=/usr/bin/sccache; \
    export CC="/usr/bin/clang"; \
    cargo auditable build --locked -p kanidm_tools ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kanidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo auditable build --locked -p kanidm-ipa-sync ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kanidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo auditable build --locked -p kanidm-ldap-sync ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kanidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo auditable build --locked -p kanidm-mail-sender ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kanidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo auditable install \
        --git https://github.com/kanidm/webauthn-rs.git \
        --rev 2ac5ff0e20f323cd904856a38a15899adf13fa5b \
        --force fido-mds-tool \
        --target-dir="/usr/src/kanidm/target/" && \
    sccache -s

FROM ${BASE_IMAGE}

ENV RUST_BACKTRACE=1

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    openssl \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder --chmod=0755 /usr/src/kanidm/target/release/kanidm /sbin/
COPY --from=builder --chmod=0755 /usr/src/kanidm/target/release/kanidm-ipa-sync /sbin/
COPY --from=builder --chmod=0755 /usr/src/kanidm/target/release/kanidm-ldap-sync /sbin/
COPY --from=builder --chmod=0755 /usr/src/kanidm/target/release/fido-mds-tool /sbin/

RUN mkdir /etc/kanidm && \
    touch /etc/kanidm/config

CMD [ "/sbin/kanidm", "-h" ]
