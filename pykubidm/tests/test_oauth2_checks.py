"""test validation of urls"""

import pytest

from kubidm import KubidmClient


def test_bad_origin() -> None:
    """testing with a bad origin"""

    client = KubidmClient(uri="http://localhost:8000")

    with pytest.raises(ValueError):
        client._validate_is_valid_origin_url("ftp://example.com")
