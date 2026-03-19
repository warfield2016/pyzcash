# pyzcash

Python bindings for [librustzcash](https://github.com/zcash/librustzcash). Covers address parsing, ZIP-321 payment URIs, and key derivation.

**Alpha quality.** The API will change.

## Install

Requires a Rust toolchain and Python 3.9+.

```
pip install maturin
git clone https://github.com/user/pyzcash && cd pyzcash
maturin develop
```

## Usage

```python
import secrets
from pyzcash import Network, ZcashAddress, Payment, TransactionRequest, UnifiedSpendingKey

# Generate a shielded address from a random seed
seed = secrets.token_bytes(32)
usk = UnifiedSpendingKey.from_seed(seed, Network.Main)
ua = usk.default_address()
print(ua.encode())  # u1...

# Parse and inspect any Zcash address
addr = ZcashAddress.parse(ua.encode())
print(addr.address_type())  # "unified"
print(addr.is_shielded())   # True

# Build a ZIP-321 payment URI
payment = Payment(addr, amount=100_000_000)  # 1 ZEC
tx = TransactionRequest.new([payment])
print(tx.to_uri())  # zcash:u1...?amount=1

# Parse a payment URI back
parsed = TransactionRequest.from_uri(tx.to_uri())
print(parsed.total())  # 100000000 (zatoshis)
```

## What it wraps

Thin PyO3 bindings over these librustzcash crates:

- **zcash_address**: parse and validate transparent, Sapling, and unified addresses
- **zip321**: ZIP-321 payment request URIs
- **zcash_keys**: unified spending key and viewing key derivation

Shielded by default: key derivation produces Orchard + Sapling receivers, no transparent component.

## API

| Class | Purpose |
|---|---|
| `Network` | `Main` or `Test` enum |
| `ZcashAddress` | Parse, validate, inspect any Zcash address |
| `Payment` | Single payment: address + amount + optional memo |
| `TransactionRequest` | ZIP-321 URI with one or more payments |
| `UnifiedSpendingKey` | Derive from seed bytes |
| `UnifiedFullViewingKey` | Encode/decode, derive addresses |
| `UnifiedAddress` | Inspect receiver types (Orchard, Sapling, transparent) |

Exceptions: `PyZcashError` (base), `AddressParseError`, `Zip321Error`, `KeyDerivationError`.

## License

MIT OR Apache-2.0
