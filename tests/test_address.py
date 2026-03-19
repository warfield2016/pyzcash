import pytest
from pyzcash import (
    Network,
    ZcashAddress,
    UnifiedSpendingKey,
    AddressParseError,
)


@pytest.fixture
def mainnet_ua():
    """Generate a mainnet unified address from a fixed seed."""
    seed = bytes(range(32))
    usk = UnifiedSpendingKey.from_seed(seed, Network.Main)
    return usk.default_address()


@pytest.fixture
def testnet_ua():
    """Generate a testnet unified address from a fixed seed."""
    seed = bytes(range(32))
    usk = UnifiedSpendingKey.from_seed(seed, Network.Test)
    return usk.default_address()


def test_parse_transparent_mainnet():
    addr = ZcashAddress.parse("t3Vz22vK5z2LcKEdg16Yv4FFneEL1zg9ojd")
    assert addr.network() == Network.Main
    assert addr.address_type() == "p2sh"
    assert not addr.is_shielded()
    assert not addr.can_receive_memo()


def test_parse_unified_mainnet(mainnet_ua):
    encoded = mainnet_ua.encode()
    addr = ZcashAddress.parse(encoded)
    assert addr.network() == Network.Main
    assert addr.address_type() == "unified"
    assert addr.is_shielded()
    assert addr.can_receive_memo()


def test_parse_unified_testnet(testnet_ua):
    encoded = testnet_ua.encode()
    addr = ZcashAddress.parse(encoded)
    assert addr.network() == Network.Test
    assert addr.address_type() == "unified"


def test_address_roundtrip(mainnet_ua):
    encoded = mainnet_ua.encode()
    addr = ZcashAddress.parse(encoded)
    assert addr.encode() == encoded


def test_address_str(mainnet_ua):
    encoded = mainnet_ua.encode()
    addr = ZcashAddress.parse(encoded)
    assert str(addr) == encoded


def test_address_eq(mainnet_ua):
    encoded = mainnet_ua.encode()
    a = ZcashAddress.parse(encoded)
    b = ZcashAddress.parse(encoded)
    assert a == b


def test_invalid_address():
    with pytest.raises(AddressParseError):
        ZcashAddress.parse("not_a_valid_address")


def test_address_repr(mainnet_ua):
    encoded = mainnet_ua.encode()
    addr = ZcashAddress.parse(encoded)
    r = repr(addr)
    assert "unified" in r
    assert encoded in r
