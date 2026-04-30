# Client Tools

To interact with Kubidm as an administrator, you'll need to use our command line tools. If you haven't installed them
yet, [install them now](installing_client_tools.md).

## Kubidm configuration

You can configure `kubidm` to help make commands simpler by modifying `~/.config/kubidm` or `/etc/kubidm/config`.

```toml
uri = "https://idm.example.com"
ca_path = "/path/to/ca.pem"
```

The full configuration reference is in the
[definition of `KubidmClientConfig`](https://kubidm.github.io/kubidm/master/rustdoc/kubidm_client/struct.KubidmClientConfig.html).

Once configured, you can test this with:

```bash
kubidm self whoami --name anonymous
```

## Session Management

To authenticate as a user (for use with the command line), you need to use the `login` command to establish a session
token.

```bash
kubidm login --name USERNAME
kubidm login --name admin
kubidm login -D USERNAME
kubidm login -D admin
```

Once complete, you can use `kubidm` without re-authenticating for a period of time for administration.

You can list active sessions with:

```bash
kubidm session list
```

Sessions will expire after a period of time. To remove these expired sessions locally you can use:

```bash
kubidm session cleanup
```

To log out of a session:

```bash
kubidm logout --name USERNAME
kubidm logout --name admin
```

## Multiple Instances

In some cases you may have multiple Kubidm instances. For example you may have a production instance and a development
instance. This can introduce friction for admins when they need to change between those instances.

The Kubidm cli tool allows you to configure multiple instances and swap between them with an environment variable, or
the `--instance` flag. Instances maintain separate session stores.

```toml
uri = "https://idm.example.com"
ca_path = "/path/to/ca.pem"

["development"]
uri = "https://idm.dev.example.com"
ca_path = "/path/to/dev-ca.pem"
```

The instance can then be selected with:

```
export KUBIDM_INSTANCE=development
kubidm login -D username@idm.dev.example.com
```

To return to the default instance you `unset` the `KUBIDM_INSTANCE` variable.
