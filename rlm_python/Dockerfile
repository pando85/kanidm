ARG BASE_IMAGE=debian:bookworm-slim

FROM ${BASE_IMAGE}

ARG RADIUS_USER=freerad
EXPOSE 1812 1813
ENV KANIDM_CONFIG_FILE="/data/radius.toml"

RUN apt-get update && apt-get install -y --no-install-recommends \
    freeradius \
    freeradius-python3 \
    freeradius-utils \
    python3 \
    python3-dev \
    python3-pip \
    iproute2 \
    iputils-ping \
    openssl \
    curl \
    hostname \
    && rm -rf /var/lib/apt/lists/*

ADD rlm_python/mods-available/ /etc/raddb/mods-available/
COPY rlm_python/sites-available/ /etc/raddb/sites-available/

WORKDIR /etc/raddb

RUN mkdir -p /etc/raddb/mods-enabled /etc/raddb/sites-enabled && \
    ln -s /etc/raddb/mods-available/python3 /etc/raddb/mods-enabled/python3 && \
    ln -s /etc/raddb/sites-available/check-eap-tls /etc/raddb/sites-enabled/check-eap-tls

RUN rm -f /etc/raddb/mods-available/sql && \
    rm -f /etc/raddb/mods-enabled/{passwd,totp}

RUN chown -R $RADIUS_USER:$RADIUS_USER /etc/raddb/ && \
    mkdir -p /etc/raddb/certs && \
    chmod 775 /etc/raddb/certs && \
    chmod 640 /etc/raddb/clients.conf

RUN mkdir -p /pkg/pykubidm/
COPY pykubidm/ /pkg/pykubidm/

RUN apt-get update && apt-get install -y --no-install-recommends gcc libssl-dev && \
    python3 -m pip install \
    --break-system-packages \
    --no-cache-dir \
    --no-warn-script-location \
    /pkg/pykubidm && \
    apt-get purge -y gcc libssl-dev && \
    apt-get autoremove -y && \
    rm -rf /var/lib/apt/lists/*

COPY rlm_python/radius_entrypoint.py /radius_entrypoint.py

RUN mkdir /data && chown $RADIUS_USER:$RADIUS_USER /data
RUN chmod a+r /etc/raddb/certs/ -R
USER $RADIUS_USER

CMD [ "/usr/bin/python3", "/radius_entrypoint.py" ]
