# RADIUS Module Development

Setting up a dev environment has some extra complexity due to the mono-repo design.

1. Install uv: `python -m pip install uv`. This is what we use to manage the packages, and allows you to set up virtual
   python environments easier.
2. Build the base environment. From within the kubidm_rlm_python directory, run: `uv sync`
3. Install the `kubidm` python library: `uv run python -m pip install ../pykubidm`
4. Start editing!

Most IDEs will be happier if you open the `kubidm_rlm_python` or `pykubidm` directories as the base you are working
from, rather than the `kubidm` repository root, so they can auto-load integrations etc.

## Running a test RADIUS container

From the root directory of the Kubidm repository:

1. Build the container - this'll give you a container image called `kubidm/radius` with the tag `devel`:

```bash
make build/radiusd
```

2. Once the process has completed, check the container exists in your docker environment:

```bash
➜ docker image ls kubidm/radius
REPOSITORY      TAG       IMAGE ID       CREATED              SIZE
kubidm/radius   devel     5dabe894134c   About a minute ago   622MB
```

_Note:_ If you're just looking to play with a pre-built container, images are also automatically built based on the
development branch and available at `ghcr.io/kubidm/radius:devel`

3. Generate some self-signed certificates by running the script - just hit enter on all the prompts if you don't want to
   customise them. This'll put the files in `/tmp/kubidm`:

```bash
./insecure_generate_tls.sh
```

4. Run the container:

```bash
cd kubidm_rlm_python && ./run_radius_container.sh
```

You can pass the following environment variables to `run_radius_container.sh` to set other options:

- IMAGE: an alternative image such as `ghcr.io/kubidm/radius:devel`
- CONFIG_FILE: mount your own config file

For example:

```bash
IMAGE=ghcr.io/kubidm/radius:devel \
    CONFIG_FILE=~/.config/kubidm \
    ./run_radius_container.sh
```

## Testing authentication

Authentication can be tested through the client.localhost Network Access Server (NAS) configuration with:

```bash
docker exec -i -t radiusd radtest \
    <username> badpassword \
    127.0.0.1 10 testing123

docker exec -i -t radiusd radtest \
    <username> <radius show_secret value here> \
    127.0.0.1 10 testing123
```
