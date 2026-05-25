# This builds the Kubidm CLI tools
ARG BASE_IMAGE=debian:trixie-slim

FROM rust:1 AS builder
ARG KANIDM_FEATURES
ARG KANIDM_BUILD_PROFILE
ARG KANIDM_BUILD_OPTIONS=""

ENV KANIDM_BUILD_PROFILE=${KANIDM_BUILD_PROFILE:-container_generic}
ENV RUSTFLAGS="-Clinker=clang"
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

<<<<<<< HEAD
RUN apt-get update && apt-get install -y --no-install-recommends \
    sccache \
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

RUN cargo install cargo-auditable
=======
RUN \
    --mount=type=cache,id=zypp,target=/var/cache/zypp \
    zypper install -y --no-recommends \
        sccache \
        cargo \
        cargo-auditable \
        clang \
        gcc \
        make \
        automake \
        autoconf \
        pam-devel \
        libudev-devel \
        sqlite3-devel \
        rsync
>>>>>>> upstream/master

COPY . /usr/src/kubidm

WORKDIR /usr/src/kubidm/

RUN \
    --mount=type=cache,id=cargo,target=/cargo \
    --mount=type=cache,id=sccache,target=/sccache \
    export CARGO_HOME=/cargo; \
    export SCCACHE_DIR=/sccache; \
    export RUSTC_WRAPPER=/usr/bin/sccache; \
    export CC="/usr/bin/clang"; \
<<<<<<< HEAD
    cargo auditable build --locked -p kubidm_tools ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kubidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo auditable build --locked -p kubidm-ipa-sync ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kubidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo auditable build --locked -p kubidm-ldap-sync ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kubidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo auditable build --locked -p kubidm-mail-sender ${KANIDM_BUILD_OPTIONS} \
=======
    cargo auditable build --locked -p kubidm_tools ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kubidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo auditable build --locked -p kubidm-ipa-sync ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kubidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo auditable build --locked -p kubidm-ldap-sync ${KANIDM_BUILD_OPTIONS} \
        --target-dir="/usr/src/kubidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo auditable build --locked -p kubidm-mail-sender ${KANIDM_BUILD_OPTIONS} \
>>>>>>> upstream/master
        --target-dir="/usr/src/kubidm/target/" \
        --features="${KANIDM_FEATURES}" \
        --release && \
    cargo auditable install \
        --git https://github.com/kubidm/webauthn-rs.git \
        --rev 2ac5ff0e20f323cd904856a38a15899adf13fa5b \
        --force fido-mds-tool \
        --target-dir="/usr/src/kubidm/target/" && \
    sccache -s

FROM ${BASE_IMAGE}

ENV RUST_BACKTRACE=1

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    openssl \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

<<<<<<< HEAD
COPY --from=builder --chmod=0755 /usr/src/kubidm/target/release/kubidm /sbin/
COPY --from=builder --chmod=0755 /usr/src/kubidm/target/release/kubidm-ipa-sync /sbin/
COPY --from=builder --chmod=0755 /usr/src/kubidm/target/release/kubidm-ldap-sync /sbin/
COPY --from=builder --chmod=0755 /usr/src/kubidm/target/release/fido-mds-tool /sbin/
=======
COPY --from=builder /usr/src/kubidm/target/release/kubidm /sbin/
COPY --from=builder /usr/src/kubidm/target/release/kubidm-ipa-sync /sbin/
COPY --from=builder /usr/src/kubidm/target/release/kubidm-ldap-sync /sbin/
COPY --from=builder /usr/src/kubidm/target/release/kubidm-mail-sender /sbin/
COPY --from=builder /usr/src/kubidm/target/release/fido-mds-tool /sbin/
RUN chmod +x /sbin/kubidm
RUN chmod +x /sbin/kubidm-ipa-sync
RUN chmod +x /sbin/kubidm-ldap-sync
RUN chmod +x /sbin/kubidm-mail-sender
RUN chmod +x /sbin/fido-mds-tool
>>>>>>> upstream/master

RUN mkdir /etc/kubidm && \
    touch /etc/kubidm/config

CMD [ "/sbin/kubidm", "-h" ]
