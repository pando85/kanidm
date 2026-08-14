#!/bin/bash

# this script runs the server in release mode and tries to set up a dev environment, which catches failures between the
# server and CLI, and ensures clap/etc rules actually work
#
# you really really really really don't want to run this when an environment you like exists, it'll mess it up

set -e

WAIT_TIMER=5
if [ -z "${BUILD_MODE}" ]; then
    BUILD_MODE="--release"
fi

echo "Building release binaries..."
# shellcheck disable=SC2086
cargo build --locked $BUILD_MODE --bin kubidm --bin kubidmd --quiet || {
    echo "Failed to build release binaries, please check the output above."
    exit 1
}

if [ ! -f "scripts/run_insecure_dev_server.sh" ]; then
    echo "I'm not sure where you are, please run this from the root of the repository"
    exit 1
fi

export KUBIDM_CONFIG="./scripts/insecure_server.toml"

mkdir -p /tmp/kubidm/client_ca

echo "Generating certificates..."
# shellcheck disable=SC2086
cargo run --bin kubidmd $BUILD_MODE cert-generate

echo "Making sure it runs with the DB..."
# shellcheck disable=SC2086
cargo run --bin kubidmd $BUILD_MODE scripting recover-account idm_admin

echo "Running the server..."
# shellcheck disable=SC2086
cargo run --bin kubidmd $BUILD_MODE server  &
KUBIDMD_PID=$!
echo "Kubidm PID: ${KUBIDMD_PID}"

if [ "$(jobs -p | wc -l)" -eq 0 ]; then
    echo "Kubidmd failed to start!"
    exit 1
fi

ATTEMPT=0

KUBIDM_CONFIG_FILE="./scripts/insecure_server.toml"
if [ -f "${KUBIDM_CONFIG_FILE}" ]; then
    echo "Found config file ${KUBIDM_CONFIG_FILE}"
else
    echo "Config file ${KUBIDM_CONFIG_FILE} not found!"
    exit 1
fi
KUBIDM_URL="$(grep -E '^origin.*https' "${KUBIDM_CONFIG_FILE}" | awk '{print $NF}' | tr -d '"')"
KUBIDM_CA_PATH="/tmp/kubidm/chain.pem"

while true; do
    echo "Waiting for the server to start... testing url '${KUBIDM_URL}'"
    curl --cacert "${KUBIDM_CA_PATH}" -f "${KUBIDM_URL}/status" >/dev/null && break
    sleep 2
    ATTEMPT="$((ATTEMPT + 1))"
    if [ "${ATTEMPT}" -gt 3 ]; then
        echo "Kubidmd failed to start!"
        exit 1
    fi
done

BUILD_MODE=$BUILD_MODE ./scripts/setup_dev_environment.sh || kill -9 "${KUBIDMD_PID}"

echo "Running the OpenAPI schema checks"

bash -c ./scripts/openapi_tests/check_openapi_spec.sh || exit 1

echo "Waiting ${WAIT_TIMER} seconds and terminating Kubidmd"
sleep "${WAIT_TIMER}"
if [ "$(pgrep kubidmd | wc -l)" -gt 0 ]; then
    kill $(pgrep kubidmd)
fi
