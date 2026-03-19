use pyo3::prelude::*;
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyValueError};

use zcash_address::ZcashAddress as RawZcashAddress;
use zcash_address::TryFromAddress;
use zcash_address::ConversionError;
use zcash_protocol::consensus::{MainNetwork, NetworkType, TestNetwork};
use zcash_protocol::memo::MemoBytes;
use zcash_protocol::value::Zatoshis;
use zip321::{Payment as RawPayment, TransactionRequest as RawTxRequest};
use zcash_keys::address::UnifiedAddress as RawUA;
use zcash_keys::keys::{
    UnifiedAddressRequest, UnifiedFullViewingKey as RawUFVK, UnifiedSpendingKey as RawUSK,
};

const ZATOSHIS_PER_ZEC: f64 = 100_000_000.0;

create_exception!(pyzcash, PyZcashError, PyException);
create_exception!(pyzcash, AddressParseError, PyZcashError);
create_exception!(pyzcash, Zip321Error, PyZcashError);
create_exception!(pyzcash, KeyDerivationError, PyZcashError);

// ── Internal helpers ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum AddressKind {
    Sprout,
    Sapling,
    Unified,
    P2pkh,
    P2sh,
    Tex,
}

struct AddressInfo {
    network: NetworkType,
    kind: AddressKind,
}

impl TryFromAddress for AddressInfo {
    type Error = ();

    fn try_from_sprout(
        net: NetworkType,
        _data: [u8; 64],
    ) -> Result<Self, ConversionError<Self::Error>> {
        Ok(AddressInfo { network: net, kind: AddressKind::Sprout })
    }

    fn try_from_sapling(
        net: NetworkType,
        _data: [u8; 43],
    ) -> Result<Self, ConversionError<Self::Error>> {
        Ok(AddressInfo { network: net, kind: AddressKind::Sapling })
    }

    fn try_from_unified(
        net: NetworkType,
        _addr: zcash_address::unified::Address,
    ) -> Result<Self, ConversionError<Self::Error>> {
        Ok(AddressInfo { network: net, kind: AddressKind::Unified })
    }

    fn try_from_transparent_p2pkh(
        net: NetworkType,
        _data: [u8; 20],
    ) -> Result<Self, ConversionError<Self::Error>> {
        Ok(AddressInfo { network: net, kind: AddressKind::P2pkh })
    }

    fn try_from_transparent_p2sh(
        net: NetworkType,
        _data: [u8; 20],
    ) -> Result<Self, ConversionError<Self::Error>> {
        Ok(AddressInfo { network: net, kind: AddressKind::P2sh })
    }

    fn try_from_tex(
        net: NetworkType,
        _data: [u8; 20],
    ) -> Result<Self, ConversionError<Self::Error>> {
        Ok(AddressInfo { network: net, kind: AddressKind::Tex })
    }
}

fn parse_raw_address(s: &str) -> PyResult<(String, NetworkType, AddressKind, bool)> {
    let addr = RawZcashAddress::try_from_encoded(s)
        .map_err(|e| AddressParseError::new_err(format!("{}", e)))?;
    let can_memo = addr.can_receive_memo();
    let info = addr
        .convert::<AddressInfo>()
        .map_err(|e| AddressParseError::new_err(format!("{:?}", e)))?;
    Ok((s.to_string(), info.network, info.kind, can_memo))
}

/// Strip trailing null bytes that Zcash uses to pad memos to 512 bytes.
fn strip_memo_padding(bytes: &[u8]) -> Vec<u8> {
    let end = bytes.iter().rposition(|&b| b != 0).map(|i| i + 1).unwrap_or(0);
    bytes[..end].to_vec()
}

// ── Network ─────────────────────────────────────────────────────────────────

#[pyclass(frozen, eq, eq_int, hash)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Network {
    Main = 0,
    Test = 1,
}

#[pymethods]
impl Network {
    fn __repr__(&self) -> &str {
        match self {
            Network::Main => "Network.Main",
            Network::Test => "Network.Test",
        }
    }
}

impl Network {
    fn to_network_type(&self) -> NetworkType {
        match self {
            Network::Main => NetworkType::Main,
            Network::Test => NetworkType::Test,
        }
    }

    fn from_network_type(nt: NetworkType) -> Self {
        match nt {
            NetworkType::Main => Network::Main,
            _ => Network::Test,
        }
    }
}

// ── ZcashAddress ────────────────────────────────────────────────────────────

#[pyclass]
#[derive(Clone)]
pub struct ZcashAddress {
    encoded: String,
    net: NetworkType,
    kind: AddressKind,
    can_memo: bool,
}

#[pymethods]
impl ZcashAddress {
    #[staticmethod]
    fn parse(s: &str) -> PyResult<Self> {
        let (encoded, net, kind, can_memo) = parse_raw_address(s)?;
        Ok(ZcashAddress { encoded, net, kind, can_memo })
    }

    fn encode(&self) -> String {
        self.encoded.clone()
    }

    fn network(&self) -> Network {
        Network::from_network_type(self.net)
    }

    fn address_type(&self) -> &str {
        match self.kind {
            AddressKind::Sprout => "sprout",
            AddressKind::Sapling => "sapling",
            AddressKind::Unified => "unified",
            AddressKind::P2pkh => "p2pkh",
            AddressKind::P2sh => "p2sh",
            AddressKind::Tex => "tex",
        }
    }

    fn is_shielded(&self) -> bool {
        matches!(
            self.kind,
            AddressKind::Sapling | AddressKind::Unified | AddressKind::Sprout
        )
    }

    fn can_receive_memo(&self) -> bool {
        self.can_memo
    }

    fn __repr__(&self) -> String {
        format!(
            "ZcashAddress('{}', type='{}', network={:?})",
            self.encoded,
            self.address_type(),
            self.network()
        )
    }

    fn __str__(&self) -> String {
        self.encoded.clone()
    }

    fn __eq__(&self, other: &ZcashAddress) -> bool {
        self.encoded == other.encoded
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.encoded.hash(&mut h);
        h.finish()
    }
}

// ── Payment ─────────────────────────────────────────────────────────────────

#[pyclass]
#[derive(Clone)]
pub struct Payment {
    address_encoded: String,
    amount_zat: u64,
    memo_data: Option<Vec<u8>>,
    label_str: Option<String>,
    message_str: Option<String>,
}

impl Payment {
    fn to_raw(&self) -> PyResult<RawPayment> {
        let addr = RawZcashAddress::try_from_encoded(&self.address_encoded)
            .map_err(|e| AddressParseError::new_err(format!("{}", e)))?;
        let amount = Zatoshis::from_u64(self.amount_zat)
            .map_err(|_| PyValueError::new_err("amount exceeds maximum (21M ZEC)"))?;
        let memo = match &self.memo_data {
            Some(b) => Some(
                MemoBytes::from_bytes(b)
                    .map_err(|_| PyValueError::new_err("memo exceeds 512 bytes"))?,
            ),
            None => None,
        };
        RawPayment::new(
            addr, amount, memo,
            self.label_str.clone(),
            self.message_str.clone(),
            vec![],
        )
        .ok_or_else(|| PyValueError::new_err("cannot attach memo to transparent address"))
    }

    fn from_raw(p: &RawPayment) -> Self {
        Payment {
            address_encoded: p.recipient_address().encode(),
            amount_zat: p.amount().into_u64(),
            memo_data: p.memo().map(|m| strip_memo_padding(m.as_slice())),
            label_str: p.label().cloned(),
            message_str: p.message().cloned(),
        }
    }
}

#[pymethods]
impl Payment {
    #[new]
    #[pyo3(signature = (address, amount, memo=None, label=None, message=None))]
    fn new(
        address: &ZcashAddress,
        amount: u64,
        memo: Option<Vec<u8>>,
        label: Option<String>,
        message: Option<String>,
    ) -> PyResult<Self> {
        Zatoshis::from_u64(amount)
            .map_err(|_| PyValueError::new_err("amount exceeds maximum (21M ZEC)"))?;
        if let Some(ref m) = memo {
            if m.len() > 512 {
                return Err(PyValueError::new_err("memo exceeds 512 bytes"));
            }
        }
        if memo.is_some() && !address.can_memo {
            return Err(PyValueError::new_err(
                "cannot attach memo to transparent address",
            ));
        }
        Ok(Payment {
            address_encoded: address.encoded.clone(),
            amount_zat: amount,
            memo_data: memo,
            label_str: label,
            message_str: message,
        })
    }

    #[getter]
    fn address(&self) -> PyResult<ZcashAddress> {
        ZcashAddress::parse(&self.address_encoded)
    }

    #[getter]
    fn amount(&self) -> u64 {
        self.amount_zat
    }

    #[getter]
    fn amount_zec(&self) -> f64 {
        self.amount_zat as f64 / ZATOSHIS_PER_ZEC
    }

    #[getter]
    fn memo(&self) -> Option<Vec<u8>> {
        self.memo_data.clone()
    }

    #[getter]
    fn memo_text(&self) -> Option<String> {
        self.memo_data.as_ref().and_then(|b| {
            let trimmed = strip_memo_padding(b);
            if trimmed.is_empty() {
                return None;
            }
            std::str::from_utf8(&trimmed).ok().map(|s| s.to_string())
        })
    }

    #[getter]
    fn label(&self) -> Option<String> {
        self.label_str.clone()
    }

    #[getter]
    fn message(&self) -> Option<String> {
        self.message_str.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "Payment(address='{}', amount={} zatoshis, {:.8} ZEC)",
            self.address_encoded, self.amount_zat, self.amount_zec()
        )
    }
}

// ── TransactionRequest ──────────────────────────────────────────────────────

#[pyclass]
#[derive(Clone)]
pub struct TransactionRequest {
    payments_list: Vec<Payment>,
}

impl TransactionRequest {
    fn build_raw(&self) -> PyResult<RawTxRequest> {
        let raw_payments: Vec<RawPayment> = self
            .payments_list
            .iter()
            .map(|p| p.to_raw())
            .collect::<PyResult<Vec<_>>>()?;
        RawTxRequest::new(raw_payments)
            .map_err(|e| Zip321Error::new_err(format!("{}", e)))
    }
}

#[pymethods]
impl TransactionRequest {
    #[staticmethod]
    fn from_uri(uri: &str) -> PyResult<Self> {
        let raw = RawTxRequest::from_uri(uri)
            .map_err(|e| Zip321Error::new_err(format!("{}", e)))?;
        let payments_list: Vec<Payment> = raw
            .payments()
            .values()
            .map(Payment::from_raw)
            .collect();
        Ok(TransactionRequest { payments_list })
    }

    #[staticmethod]
    fn new(payments: Vec<Payment>) -> PyResult<Self> {
        if payments.is_empty() {
            return Err(PyValueError::new_err("at least one payment is required"));
        }
        let tx = TransactionRequest { payments_list: payments };
        tx.build_raw()?;
        Ok(tx)
    }

    fn to_uri(&self) -> PyResult<String> {
        Ok(self.build_raw()?.to_uri())
    }

    #[getter]
    fn payments(&self) -> Vec<Payment> {
        self.payments_list.clone()
    }

    fn total(&self) -> PyResult<u64> {
        let total = self.build_raw()?
            .total()
            .map_err(|e| PyValueError::new_err(format!("balance error: {}", e)))?;
        Ok(total.into_u64())
    }

    fn total_zec(&self) -> PyResult<f64> {
        Ok(self.total()? as f64 / ZATOSHIS_PER_ZEC)
    }

    fn __len__(&self) -> usize {
        self.payments_list.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "TransactionRequest({} payment(s))",
            self.payments_list.len()
        )
    }

    fn __str__(&self) -> PyResult<String> {
        self.to_uri()
    }
}

// ── Key derivation ──────────────────────────────────────────────────────────

#[pyclass]
pub struct UnifiedSpendingKey {
    inner: RawUSK,
    net: NetworkType,
}

#[pymethods]
impl UnifiedSpendingKey {
    #[staticmethod]
    #[pyo3(signature = (seed, network, account=0))]
    fn from_seed(seed: Vec<u8>, network: &Network, account: u32) -> PyResult<Self> {
        let net = network.to_network_type();
        let account_id = zip32::AccountId::try_from(account)
            .map_err(|_| PyValueError::new_err("account index must be < 2^31"))?;

        let usk = match net {
            NetworkType::Main => RawUSK::from_seed(&MainNetwork, &seed, account_id),
            _ => RawUSK::from_seed(&TestNetwork, &seed, account_id),
        }
        .map_err(|e| KeyDerivationError::new_err(format!("{}", e)))?;

        Ok(UnifiedSpendingKey { inner: usk, net })
    }

    fn to_unified_full_viewing_key(&self) -> UnifiedFullViewingKey {
        UnifiedFullViewingKey {
            inner: self.inner.to_unified_full_viewing_key(),
            net: self.net,
        }
    }

    fn default_address(&self) -> PyResult<UnifiedAddress> {
        let ufvk = self.inner.to_unified_full_viewing_key();
        let (ua, _) = ufvk
            .default_address(UnifiedAddressRequest::SHIELDED)
            .map_err(|e| KeyDerivationError::new_err(format!("{}", e)))?;
        Ok(UnifiedAddress { inner: ua, net: self.net })
    }

    fn __repr__(&self) -> String {
        format!(
            "UnifiedSpendingKey(network={:?})",
            Network::from_network_type(self.net)
        )
    }
}

#[pyclass]
pub struct UnifiedFullViewingKey {
    inner: RawUFVK,
    net: NetworkType,
}

#[pymethods]
impl UnifiedFullViewingKey {
    fn encode(&self) -> String {
        match self.net {
            NetworkType::Main => self.inner.encode(&MainNetwork),
            _ => self.inner.encode(&TestNetwork),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (encoded, network))]
    fn decode(encoded: &str, network: &Network) -> PyResult<Self> {
        let net = network.to_network_type();
        let ufvk = match net {
            NetworkType::Main => RawUFVK::decode(&MainNetwork, encoded),
            _ => RawUFVK::decode(&TestNetwork, encoded),
        }
        .map_err(|e| KeyDerivationError::new_err(format!("{}", e)))?;
        Ok(UnifiedFullViewingKey { inner: ufvk, net })
    }

    fn default_address(&self) -> PyResult<UnifiedAddress> {
        let (ua, _) = self
            .inner
            .default_address(UnifiedAddressRequest::SHIELDED)
            .map_err(|e| KeyDerivationError::new_err(format!("{}", e)))?;
        Ok(UnifiedAddress { inner: ua, net: self.net })
    }

    fn network(&self) -> Network {
        Network::from_network_type(self.net)
    }

    fn __repr__(&self) -> String {
        format!(
            "UnifiedFullViewingKey(network={:?})",
            Network::from_network_type(self.net)
        )
    }
}

#[pyclass]
pub struct UnifiedAddress {
    inner: RawUA,
    net: NetworkType,
}

#[pymethods]
impl UnifiedAddress {
    fn encode(&self) -> String {
        match self.net {
            NetworkType::Main => self.inner.encode(&MainNetwork),
            _ => self.inner.encode(&TestNetwork),
        }
    }

    fn to_zcash_address(&self) -> ZcashAddress {
        let raw = self.inner.to_zcash_address(self.net);
        ZcashAddress {
            encoded: raw.encode(),
            net: self.net,
            kind: AddressKind::Unified,
            can_memo: true,
        }
    }

    fn has_orchard(&self) -> bool {
        self.inner.has_orchard()
    }

    fn has_sapling(&self) -> bool {
        self.inner.has_sapling()
    }

    fn has_transparent(&self) -> bool {
        self.inner.has_transparent()
    }

    fn network(&self) -> Network {
        Network::from_network_type(self.net)
    }

    fn __repr__(&self) -> String {
        format!("UnifiedAddress('{}')", self.encode())
    }

    fn __str__(&self) -> String {
        self.encode()
    }
}

// ── Module ──────────────────────────────────────────────────────────────────

#[pymodule]
fn _pyzcash(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Network>()?;
    m.add_class::<ZcashAddress>()?;
    m.add_class::<Payment>()?;
    m.add_class::<TransactionRequest>()?;
    m.add_class::<UnifiedSpendingKey>()?;
    m.add_class::<UnifiedFullViewingKey>()?;
    m.add_class::<UnifiedAddress>()?;
    m.add("PyZcashError", m.py().get_type::<PyZcashError>())?;
    m.add("AddressParseError", m.py().get_type::<AddressParseError>())?;
    m.add("Zip321Error", m.py().get_type::<Zip321Error>())?;
    m.add("KeyDerivationError", m.py().get_type::<KeyDerivationError>())?;
    Ok(())
}
