# set the following environment variables
# OAUTH_HOSTNAME       - the hostname you'll be exposing this as
# OAUTH_PORT           - the external port this'll be running on (sets it in --publish)
#                        this is 8553, but ... things get weird if the stack doesn't end up being accessed
#                        through 443, eg via a tunneled proxy
# KUBIDM_HOSTNAME      - the hostname of the Kubidm instance
# KUBIDM_PORT          - if you're running it on a different port
# KUBIDM_CLIENT_SECRET - the client secret for the RP in Kubidm's OAuth config
KUBIDM_PORT ?= 443
OAUTH_PORT ?= 8553

.DEFAULT: build_and_run

.PHONY: build_and_run
build_and_run: build run

.PHONY: build
build:
	docker build -t kubidm_oauth_test:latest .

.PHONY: run
run:
	docker rm -f kubidm_oauth_test
	docker run --rm -it \
		--env OAUTH_HOSTNAME=$(OAUTH_HOSTNAME) \
		--env KUBIDM_HOSTNAME=$(KUBIDM_HOSTNAME) \
		--env KUBIDM_PORT=$(KUBIDM_PORT) \
		--env KUBIDM_CLIENT_SECRET=$(KUBIDM_CLIENT_SECRET) \
		--volume /tmp/kubidm/:/certs/ \
		--publish "$(OAUTH_PORT):443" \
		--name kubidm_oauth_test \
		kubidm_oauth_test:latest
