import pytest
from pyzcash import (
    Network,
    ZcashAddress,
    Payment,
    TransactionRequest,
    UnifiedSpendingKey,
    Zip321Error,
)


@pytest.fixture
def shielded_addr():
    """A shielded unified address for testing."""
    seed = bytes(range(32))
    usk = UnifiedSpendingKey.from_seed(seed, Network.Main)
    ua = usk.default_address()
    return ZcashAddress.parse(ua.encode())


def test_payment_creation(shielded_addr):
    p = Payment(shielded_addr, 100_000_000)
    assert p.amount == 100_000_000
    assert p.amount_zec == 1.0
    assert p.memo is None
    assert p.label is None
    assert p.message is None


def test_payment_with_memo(shielded_addr):
    memo = b"Hello Zcash"
    p = Payment(shielded_addr, 50_000_000, memo=memo)
    assert p.memo == memo
    assert p.memo_text == "Hello Zcash"


def test_payment_with_label_message(shielded_addr):
    p = Payment(shielded_addr, 1_000_000, label="Donation", message="Thanks")
    assert p.label == "Donation"
    assert p.message == "Thanks"


def test_transaction_request_roundtrip(shielded_addr):
    p = Payment(shielded_addr, 100_000_000)
    tx = TransactionRequest.new([p])
    uri = tx.to_uri()
    assert uri.startswith("zcash:")
    tx2 = TransactionRequest.from_uri(uri)
    assert len(tx2) == 1
    assert tx2.total() == 100_000_000


def test_transaction_request_total(shielded_addr):
    p1 = Payment(shielded_addr, 100_000_000)
    p2 = Payment(shielded_addr, 50_000_000)
    tx = TransactionRequest.new([p1, p2])
    assert tx.total() == 150_000_000
    assert tx.total_zec() == 1.5


def test_transaction_request_payments(shielded_addr):
    p = Payment(shielded_addr, 100_000_000)
    tx = TransactionRequest.new([p])
    payments = tx.payments
    assert len(payments) == 1
    assert payments[0].amount == 100_000_000


def test_transaction_request_str(shielded_addr):
    p = Payment(shielded_addr, 100_000_000)
    tx = TransactionRequest.new([p])
    assert str(tx).startswith("zcash:")


def test_invalid_uri():
    with pytest.raises(Zip321Error):
        TransactionRequest.from_uri("not_a_valid_uri")


def test_empty_payments():
    with pytest.raises(ValueError):
        TransactionRequest.new([])


def test_memo_roundtrip_through_uri(shielded_addr):
    """Memo bytes should survive a URI round-trip without null padding."""
    original_memo = b"Hello Zcash"
    p = Payment(shielded_addr, 50_000_000, memo=original_memo)
    tx = TransactionRequest.new([p])
    uri = tx.to_uri()
    parsed = TransactionRequest.from_uri(uri)
    recovered = parsed.payments[0]
    assert recovered.memo == original_memo
    assert recovered.memo_text == "Hello Zcash"


def test_memo_on_transparent():
    addr = ZcashAddress.parse("t3Vz22vK5z2LcKEdg16Yv4FFneEL1zg9ojd")
    with pytest.raises(ValueError, match="memo"):
        Payment(addr, 100_000_000, memo=b"test")
