"""tests the check_vlan function"""

import asyncio
from typing import Any

import pytest

from kubidm import KubidmClient
from kubidm.types import KubidmClientConfig, RadiusTokenGroup

from kubidm.radius.utils import check_vlan


@pytest.mark.asyncio
async def test_check_vlan() -> None:
    """test 1"""

    # event_loop = asyncio.get_running_loop()

    testconfig = KubidmClientConfig.parse_toml(
        """
    uri='https://kubidm.example.com'
    radius_groups = [
        { spn = "crabz@example.com", "vlan" = 1234 },
        { spn = "hello@world", "vlan" = 12345 },
    ]
    """
    )

    print(f"{testconfig=}")

    kubidm_client = KubidmClient(
        config=testconfig,
    )
    print(f"{kubidm_client.config=}")

    assert (
        check_vlan(
            acc=12345678,
            group=RadiusTokenGroup(spn="crabz@example.com", uuid="crabz"),
            kubidm_client=kubidm_client,
        )
        == 1234
    )

    assert (
        check_vlan(
            acc=12345678,
            group=RadiusTokenGroup(spn="foo@bar.com", uuid="lol"),
            kubidm_client=kubidm_client,
        )
        == 12345678
    )
