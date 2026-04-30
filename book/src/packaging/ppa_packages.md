# Kubidm PPA Packages

> [!NOTE]
> These are not supported in the main repo, please raise issues in the
> [kubidm/kubidm_ppa_automation](https://github.com/kubidm/kubidm_ppa_automation) repository, and understand that it is
> a community-supported effort, rather than by the core Kubidm project.

- The Kubidm PPA repository contains Debian & Ubuntu packages built from the
  [main Kubidm repository](https://github.com/kubidm/kubidm).
- Two separate components are available, `stable` for released versions and `nightly` which only provides the latest
  bleeding edge, refreshed once a day.
- Packages are distributed for current LTS versions of Debian & Ubuntu that natively package the required dependencies;
  - Ubuntu: 22.04 aka `jammy` & 24.04 aka `noble`.
  - Debian 12 aka `bookworm`.

- Please note that while the spirit of the commands below should also work on other Debian-based distributions, the
  codename detection will not work and you will need to manually choose which distribution is the closest to yours. The
  methods for adding repositories may also vary, for example Pop OS, requires an altered setup in line with their
  [instructions](https://support.system76.com/articles/ppa-third-party/).

## Adding it to your system

Make sure you have a “trusted GPG” directory for storing signing keys.

```bash
sudo mkdir -p /etc/apt/trusted.gpg.d/
```

Download the Kubidm PPA GPG public key.

```bash
curl -s "https://kubidm.github.io/kubidm_ppa/kubidm_ppa.asc" \
    | sudo tee /etc/apt/trusted.gpg.d/kubidm_ppa.asc >/dev/null
```

Add the Kubidm PPA to your local APT configuration, with autodetection of Ubuntu vs. Debian. Please adjust accordingly
if you want the `nightly` component instead of the default `stable`.

```bash
curl -s "https://kubidm.github.io/kubidm_ppa/kubidm_ppa.list" \
    | grep $( ( . /etc/os-release && echo $VERSION_CODENAME) ) | grep stable \
    | sudo tee /etc/apt/sources.list.d/kubidm_ppa.list
```

Update your local package cache.

```bash
sudo apt update
```

## Listing Packages

Use `apt search` to list the packages available:

```bash
apt search kubidm
```

## Installing stable on top of nightly

If you previously had the alpha version kubidm nightly packages installed or are switching from nightly down to stable,
it may be difficult to remove the previous versions safely without losing for example Kubidm backed sudo in the middle.
This snippet is intended to help with that:

```bash
sudo bash <<EOT
dpkg --remove kubidm kubidm-unixd libnss-kubidm libpam-kubidm
apt install -y kubidm kubidm-unixd
EOT
```

If anything goes wrong during the snippet, you may need to fall back to other methods of gaining root to complete the
transition!
