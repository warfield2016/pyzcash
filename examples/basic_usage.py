import secrets
from pyzcash import (
    Network,
    ZcashAddress,
    Payment,
    TransactionRequest,
    UnifiedSpendingKey,
)

# Generate a new wallet address
seed = secrets.token_bytes(32)
usk = UnifiedSpendingKey.from_seed(seed, Network.Main, account=0)
ufvk = usk.to_unified_full_viewing_key()
ua = usk.default_address()

print("Generated unified address:", ua.encode())
print("  Has Orchard receiver:", ua.has_orchard())
print("  Has Sapling receiver:", ua.has_sapling())
print("  Has transparent receiver:", ua.has_transparent())
print()

# Parse and inspect an address
addr = ZcashAddress.parse(ua.encode())
print("Parsed address type:", addr.address_type())
print("  Network:", addr.network())
print("  Shielded:", addr.is_shielded())
print("  Can receive memo:", addr.can_receive_memo())
print()

# Build a ZIP-321 payment URI
payment = Payment(addr, amount=100_000_000, memo=b"Hello from pyzcash")
print("Payment:", payment.amount_zec, "ZEC")
print("  Memo:", payment.memo_text)

tx = TransactionRequest.new([payment])
print("  URI:", tx.to_uri()[:100], "...")
print()

# Parse a payment URI
uri = tx.to_uri()
parsed_tx = TransactionRequest.from_uri(uri)
print("Parsed URI:", len(parsed_tx), "payment(s)")
print("  Total:", parsed_tx.total_zec(), "ZEC")

# Encode and decode a full viewing key
encoded_fvk = ufvk.encode()
print()
print("UFVK:", encoded_fvk[:60], "...")
decoded_fvk = UnifiedSpendingKey.from_seed(seed, Network.Main).to_unified_full_viewing_key()
assert decoded_fvk.encode() == encoded_fvk
print("UFVK round-trip: OK")
