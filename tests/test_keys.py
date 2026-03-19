import pytest
from pyzcash import (
    Network,
    ZcashAddress,
    UnifiedSpendingKey,
    UnifiedFullViewingKey,
    KeyDerivationError,
)

FIXED_SEED = bytes(range(32))


def test_usk_from_seed():
    usk = UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Main)
    assert repr(usk) == "UnifiedSpendingKey(network=Main)"


def test_usk_deterministic():
    usk1 = UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Main)
    usk2 = UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Main)
    addr1 = usk1.default_address().encode()
    addr2 = usk2.default_address().encode()
    assert addr1 == addr2


def test_usk_different_seeds():
    seed1 = bytes(range(32))
    seed2 = bytes(range(1, 33))
    addr1 = UnifiedSpendingKey.from_seed(seed1, Network.Main).default_address().encode()
    addr2 = UnifiedSpendingKey.from_seed(seed2, Network.Main).default_address().encode()
    assert addr1 != addr2


def test_usk_different_networks():
    addr_main = UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Main).default_address().encode()
    addr_test = UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Test).default_address().encode()
    assert addr_main != addr_test
    assert addr_main.startswith("u1")
    assert addr_test.startswith("utest")


def test_usk_different_accounts():
    addr0 = UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Main, account=0).default_address().encode()
    addr1 = UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Main, account=1).default_address().encode()
    assert addr0 != addr1


def test_ufvk_encode_decode():
    usk = UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Main)
    ufvk = usk.to_unified_full_viewing_key()
    encoded = ufvk.encode()
    assert encoded.startswith("uview")

    ufvk2 = UnifiedFullViewingKey.decode(encoded, Network.Main)
    assert ufvk2.encode() == encoded


def test_ufvk_default_address():
    usk = UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Main)
    ufvk = usk.to_unified_full_viewing_key()
    ua = ufvk.default_address()
    assert ua.has_orchard()
    assert ua.has_sapling()
    assert not ua.has_transparent()


def test_ua_to_zcash_address():
    usk = UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Main)
    ua = usk.default_address()
    addr = ua.to_zcash_address()
    assert addr.address_type() == "unified"
    assert addr.is_shielded()
    assert addr.encode() == ua.encode()


def test_ua_roundtrip():
    usk = UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Main)
    ua = usk.default_address()
    encoded = ua.encode()
    parsed = ZcashAddress.parse(encoded)
    assert parsed.encode() == encoded


def test_invalid_account():
    with pytest.raises(ValueError, match="2\\^31"):
        UnifiedSpendingKey.from_seed(FIXED_SEED, Network.Main, account=2**31)
