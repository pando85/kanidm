#!/bin/bash
set -x


if [ -z "${IMAGE}" ]; then
    IMAGE="kubidm/radius:devel"
fi
echo "Running docker container: ${IMAGE}"

if [ -n "${IMAGE_ARCH}" ]; then
    IMAGE_ARCH="--platform ${IMAGE_ARCH}"
fi

if [ -z "${CONFIG_FILE}" ]; then
    CONFIG_FILE="$(pwd)/../examples/radius.toml"
fi
echo "Using config file: ${CONFIG_FILE}"

if [ ! -d "/tmp/kubidm/" ]; then
	echo "Can't find /tmp/kubidm - you may need to run run_insecure_dev_server"
fi

echo "Starting the dev container..."
#shellcheck disable=SC2068
docker run --rm -it \
    "${IMAGE_ARCH}" \
    --network host \
    --name radiusd \
    -v /tmp/kubidm/:/data/ \
    -v /tmp/kubidm/:/tmp/kubidm/ \
    -v /tmp/kubidm/:/certs/ \
    -v "${CONFIG_FILE}:/data/radius.toml" \
    "${IMAGE}" $@
