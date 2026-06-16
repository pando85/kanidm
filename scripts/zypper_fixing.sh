#!/bin/bash

# makes sure the repos are configured because the containers are derpy sometimes

set -e

# Increase curl timeout for slow mirrors (15 minutes total, 3 min connect)
export ZYPP_LIBCURL_CURLTIMEOUT=900
export ZYPP_LIBCURL_CONNECTTIMEOUT=180

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
# retry up to 10 times with a 60 second delay for transient network issues
# also use zypper's internal download retries
MAX_RETRIES=10
RETRY_DELAY=60
ZYPPER_RETRY_OPTS="--download-retries-limit 5 --download-retry-delay 30"

for i in $(seq 1 $MAX_RETRIES); do
    echo "Repository refresh attempt $i of $MAX_RETRIES"
    # Try without --force first (uses cached metadata if available)
    if zypper ref $ZYPPER_RETRY_OPTS; then
        echo "Repository refresh succeeded (using cache)"
        break
    fi
    # If cached refresh fails, try with --force
    echo "Cached refresh failed, trying with --force..."
    if zypper ref --force $ZYPPER_RETRY_OPTS; then
        echo "Repository refresh succeeded (forced)"
        break
    fi
    if [ "$i" -lt $MAX_RETRIES ]; then
        echo "Repository refresh failed, waiting ${RETRY_DELAY}s before retry..."
        sleep $RETRY_DELAY
    else
        echo "Repository refresh failed after $MAX_RETRIES attempts"
        exit 1
    fi
done

# show which mirror is failing if an error occurs (otherwise zypper shows the wrong mirror url)
zypper -v dup -y
