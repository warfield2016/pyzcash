import http.server
import json
import secrets
import urllib.parse
import os

from pyzcash import (
    Network,
    ZcashAddress,
    Payment,
    TransactionRequest,
    UnifiedSpendingKey,
    AddressParseError,
    Zip321Error,
)

PORT = int(os.environ.get("PORT", "8080"))
DEMO_DIR = os.path.dirname(os.path.abspath(__file__))


def handle_api(path, params):
    if path == "/api/generate":
        network = Network.Main if params.get("network", "main") == "main" else Network.Test
        account = int(params.get("account", "0"))
        seed = secrets.token_bytes(32)
        usk = UnifiedSpendingKey.from_seed(seed, network, account)
        ua = usk.default_address()
        ufvk = usk.to_unified_full_viewing_key()
        return {
            "address": ua.encode(),
            "has_orchard": ua.has_orchard(),
            "has_sapling": ua.has_sapling(),
            "has_transparent": ua.has_transparent(),
            "ufvk": ufvk.encode(),
            "seed_hex": seed.hex(),
        }

    elif path == "/api/parse":
        addr_str = params.get("address", "")
        try:
            addr = ZcashAddress.parse(addr_str)
            return {
                "valid": True,
                "type": addr.address_type(),
                "network": "main" if addr.network() == Network.Main else "test",
                "shielded": addr.is_shielded(),
                "can_receive_memo": addr.can_receive_memo(),
            }
        except AddressParseError as e:
            return {"valid": False, "error": str(e)}

    elif path == "/api/payment-uri":
        addr_str = params.get("address", "")
        amount = int(params.get("amount", "0"))
        memo = params.get("memo", "")
        try:
            addr = ZcashAddress.parse(addr_str)
            memo_bytes = memo.encode("utf-8") if memo else None
            payment = Payment(addr, amount, memo=memo_bytes)
            tx = TransactionRequest.new([payment])
            return {"uri": tx.to_uri(), "total_zec": tx.total_zec()}
        except (AddressParseError, Zip321Error, ValueError) as e:
            return {"error": str(e)}

    elif path == "/api/parse-uri":
        uri = params.get("uri", "")
        try:
            tx = TransactionRequest.from_uri(uri)
            payments = []
            for p in tx.payments:
                payments.append({
                    "address": p.address.encode(),
                    "amount_zec": p.amount_zec,
                    "memo": p.memo_text,
                    "shielded": p.address.is_shielded(),
                })
            return {"payments": payments, "total_zec": tx.total_zec()}
        except Zip321Error as e:
            return {"error": str(e)}

    return {"error": "unknown endpoint"}


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=DEMO_DIR, **kwargs)

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path.startswith("/api/"):
            params = dict(urllib.parse.parse_qsl(parsed.query))
            result = handle_api(parsed.path, params)
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps(result).encode())
        else:
            if parsed.path == "/":
                self.path = "/index.html"
            super().do_GET()

    def log_message(self, format, *args):
        pass


if __name__ == "__main__":
    with http.server.HTTPServer(("", PORT), Handler) as s:
        print(f"Demo running on http://localhost:{PORT}")
        s.serve_forever()
