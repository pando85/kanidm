ARG BASE_IMAGE=debian:trixie-slim

FROM ${BASE_IMAGE}

ARG RADIUS_USER=freerad
EXPOSE 1812 1813
ENV KANIDM_CONFIG_FILE="/data/kanidm"

RUN apt-get update && apt-get install -y --no-install-recommends \
    freeradius \
    freeradius-python3 \
    python3 \
    python3-pip \
    python3-dev \
    openssl \
    curl \
    iproute2 \
    iputils-ping \
    && rm -rf /var/lib/apt/lists/*

# Debian uses /etc/freeradius, but our configs reference /etc/raddb
RUN ln -s /etc/freeradius /etc/raddb

ADD rlm_python/mods-available/ /etc/freeradius/mods-available/
COPY rlm_python/sites-available/ /etc/freeradius/sites-available/

WORKDIR /etc/freeradius

RUN mkdir -p /etc/freeradius/mods-enabled /etc/freeradius/sites-enabled && \
    ln -sf /etc/freeradius/mods-available/python3 /etc/freeradius/mods-enabled/python3 && \
    ln -sf /etc/freeradius/sites-available/check-eap-tls /etc/freeradius/sites-enabled/check-eap-tls

RUN rm -f /etc/freeradius/mods-available/sql && \
    rm -f /etc/freeradius/mods-enabled/passwd && \
    rm -f /etc/freeradius/mods-enabled/totp

RUN chown -R $RADIUS_USER:$RADIUS_USER /etc/freeradius/ && \
    mkdir -p /etc/freeradius/certs && \
    chmod 775 /etc/freeradius/certs && \
    chmod 640 /etc/freeradius/clients.conf

RUN mkdir -p /pkg/pykanidm/
COPY pykanidm/ /pkg/pykanidm/

RUN apt-get update && apt-get install -y --no-install-recommends gcc libssl-dev && \
    python3 -m pip install --break-system-packages --no-cache-dir /pkg/pykanidm && \
    apt-get purge -y gcc libssl-dev && \
    apt-get autoremove -y && \
    rm -rf /var/lib/apt/lists/*

COPY rlm_python/radius_entrypoint.py /radius_entrypoint.py

RUN mkdir /data && chown $RADIUS_USER /data
RUN chmod a+r /etc/freeradius/certs/ -R
USER $RADIUS_USER

CMD [ "/usr/bin/python3", "/radius_entrypoint.py" ]
