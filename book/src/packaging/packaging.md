# Packages

This chapter presents the alternative packages and how to build your own.

To ease packaging for your distribution, the `Makefile` has targets for sets of binary outputs.

| Target                 | Description                 |
| ---------------------- | --------------------------- |
| `release/kubidm`       | Kubidm's CLI                |
| `release/kubidmd`      | The server daemon           |
| `release/kubidm-ssh`   | SSH-related utilities       |
| `release/kubidm-unixd` | UNIX tools, PAM/NSS modules |

## Community Packages

There are several community maintained packages that you may use in your system. However, they are not officially
supported and may not function identically.

- [Alpine Linux](https://pkgs.alpinelinux.org/packages?name=kubidm%2A)
- [Arch Linux](https://aur.archlinux.org/packages?O=0&K=kubidm)
- [Debian / Ubuntu](debian_ubuntu_packaging.md)
- [NixOS](https://search.nixos.org/packages?sort=relevance&type=packages&query=kubidm)
- [OpenSUSE](https://software.opensuse.org/search?baseproject=ALL&q=kubidm)
