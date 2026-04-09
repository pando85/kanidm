""" class utils """

from typing import Optional
import logging
import os

from .. import KubidmClient
from ..types import RadiusTokenGroup


def check_vlan(
    acc: int,
    group: RadiusTokenGroup,
    kubidm_client: Optional[KubidmClient] = None,
) -> int:
    """checks if a vlan is in the config,

    acc is the default vlan
    """
    logging.debug("acc=%s", acc)
    if kubidm_client is None:
        if "KANIDM_CONFIG_FILE" in os.environ:
            kubidm_client = KubidmClient(config_file=os.environ["KANIDM_CONFIG_FILE"])
        else:
            raise ValueError("Need to pass this a kubidm_client")

    for radius_group in kubidm_client.config.radius_groups:
        logging.debug(
            "Checking vlan group '%s' against user group %s",
            radius_group.spn,
            group.spn,
        )
        if radius_group.spn == group.spn:
            logging.info("returning new vlan: %s", radius_group.vlan)
            return radius_group.vlan
    logging.debug("returning already set vlan: %s", acc)
    return acc
