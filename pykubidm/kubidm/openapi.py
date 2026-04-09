"""Helpers for the OpenAPI-generated client."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from . import KubidmClient
from .types import KubidmClientConfig

try:
    from kubidm_openapi_client import ApiClient, Configuration
except ImportError as exc:  # pragma: no cover - packaged with project
    raise ImportError("kubidm_openapi_client is not available; re-run OpenAPI codegen") from exc


def openapi_configuration_from_client_config(config: KubidmClientConfig) -> Configuration:
    """Create an OpenAPI Configuration from a KubidmClientConfig."""
    if config.uri is None:
        raise ValueError("KubidmClientConfig.uri must be set")

    host = config.uri.rstrip("/")
    configuration = Configuration(host=host)

    verify_ssl = config.verify_certificate and config.verify_ca
    configuration.verify_ssl = verify_ssl
    setattr(configuration, "assert_hostname", config.verify_hostnames)
    if config.ca_path is not None:
        configuration.ssl_ca_cert = config.ca_path
    if config.auth_token is not None:
        configuration.access_token = config.auth_token

    return configuration


def openapi_client_from_client_config(config: KubidmClientConfig) -> ApiClient:
    """Create an OpenAPI ApiClient from a KubidmClientConfig."""
    return ApiClient(configuration=openapi_configuration_from_client_config(config))


def openapi_client_from_kubidm_client(client: "KubidmClient") -> ApiClient:
    """Create an OpenAPI ApiClient from a KubidmClient instance."""
    return openapi_client_from_client_config(client.config)


__all__ = [
    "ApiClient",
    "Configuration",
    "openapi_client_from_client_config",
    "openapi_client_from_kubidm_client",
    "openapi_configuration_from_client_config",
]
