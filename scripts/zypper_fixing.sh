#!/bin/bash

# makes sure the repos are configured because the containers are derpy sometimes

set -e

# Increase curl timeout for slow mirrors (5 minutes)
export ZYPP_LIBCURL_CURLTIMEOUT=300
export ZYPP_LIBCURL_CONNECTTIMEOUT=60

#disable the openh264 repo
if [ "$(zypper lr | grep -ci 'repo-openh264')" -eq 1 ]; then
    echo "Disabling openh264 repo"
    zypper mr -d -f repo-openh264
fi

# add the non-oss repo if it doesn't exist
echo "Adding the non-oss repo"
if [ "$(zypper lr | grep -c 'repo-non-oss')" -eq 0 ]; then
    zypper ar -f -n 'Non-OSS' http://download.opensuse.org/tumbleweed/repo/non-oss/ repo-non-oss
fi

# update the repos and make sure the ones we want are enabled
zypper mr -k repo-oss
zypper mr -k repo-non-oss
zypper mr -k repo-update

# force the refresh because zypper is too silly to work out it needs to do it itself
# retry up to 5 times with exponential backoff for transient network issues
MAX_RETRIES=5
BASE_DELAY=15
for i in $(seq 1 $MAX_RETRIES); do
    echo "Repository refresh attempt $i of $MAX_RETRIES"
    if zypper ref --force; then
        echo "Repository refresh succeeded"
        break
    fi
    if [ "$i" -lt $MAX_RETRIES ]; then
        DELAY=$((BASE_DELAY * (2 ** (i - 1))))
        echo "Repository refresh failed, waiting ${DELAY}s before retry..."
        sleep $DELAY
        if [ "$i" -ge 2 ]; then
            echo "Trying alternate mirror..."
            zypper mr -f -U "http://download.opensuse.org/tumbleweed/repo/oss/" repo-oss 2>/dev/null || true
        fi
    else
        echo "Repository refresh failed after $MAX_RETRIES attempts"
        exit 1
    fi
done

# show which mirror is failing if an error occurs (otherwise zypper shows the wrong mirror url)
zypper -v dup -y
