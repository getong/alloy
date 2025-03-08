use crate::{
    transaction::{
        eip4844::{TxEip4844, TxEip4844Variant, TxEip4844WithSidecar},
        RlpEcdsaEncodableTx,
    },
    EthereumTxEnvelope, SignableTransaction, Transaction, TxEip1559, TxEip2930, TxEip7702,
    TxLegacy, TxType,
};
use alloy_eips::{eip2930::AccessList, eip7702::SignedAuthorization, Typed2718};
use alloy_primitives::{
    bytes::BufMut, Bytes, ChainId, PrimitiveSignature as Signature, TxHash, TxKind, B256, U256,
};

/// Basic typed transaction which can contain both [`TxEip4844`] and [`TxEip4844WithSidecar`].
pub type TypedTransaction = EthereumTypedTransaction<TxEip4844Variant>;

/// The TypedTransaction enum represents all Ethereum transaction request types.
///
/// Its variants correspond to specific allowed transactions:
/// 1. Legacy (pre-EIP2718) [`TxLegacy`]
/// 2. EIP2930 (state access lists) [`TxEip2930`]
/// 3. EIP1559 [`TxEip1559`]
/// 4. EIP4844 [`TxEip4844Variant`]
///
/// This type is generic over Eip4844 variant to support the following cases:
/// 1. Only-[`TxEip4844`] transaction type, such transaction representation is returned by RPC and
///    stored by nodes internally.
/// 2. Only-[`TxEip4844WithSidecar`] transactions which are broadcasted over the network, submitted
///    to RPC and stored in transaction pool.
/// 3. Dynamic [`TxEip4844Variant`] transactions to support both of the above cases via a single
///    type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        from = "serde_from::MaybeTaggedTypedTransaction<Eip4844>",
        into = "serde_from::TaggedTypedTransaction<Eip4844>",
        bound = "Eip4844: Clone + serde::Serialize + serde::de::DeserializeOwned"
    )
)]
#[cfg_attr(all(any(test, feature = "arbitrary"), feature = "k256"), derive(arbitrary::Arbitrary))]
#[doc(alias = "TypedTx", alias = "TxTyped", alias = "TransactionTyped")]
pub enum EthereumTypedTransaction<Eip4844> {
    /// Legacy transaction
    #[cfg_attr(feature = "serde", serde(rename = "0x00", alias = "0x0"))]
    Legacy(TxLegacy),
    /// EIP-2930 transaction
    #[cfg_attr(feature = "serde", serde(rename = "0x01", alias = "0x1"))]
    Eip2930(TxEip2930),
    /// EIP-1559 transaction
    #[cfg_attr(feature = "serde", serde(rename = "0x02", alias = "0x2"))]
    Eip1559(TxEip1559),
    /// EIP-4844 transaction
    #[cfg_attr(feature = "serde", serde(rename = "0x03", alias = "0x3"))]
    Eip4844(Eip4844),
    /// EIP-7702 transaction
    #[cfg_attr(feature = "serde", serde(rename = "0x04", alias = "0x4"))]
    Eip7702(TxEip7702),
}

impl<Eip4844> From<TxLegacy> for EthereumTypedTransaction<Eip4844> {
    fn from(tx: TxLegacy) -> Self {
        Self::Legacy(tx)
    }
}

impl<Eip4844> From<TxEip2930> for EthereumTypedTransaction<Eip4844> {
    fn from(tx: TxEip2930) -> Self {
        Self::Eip2930(tx)
    }
}

impl<Eip4844> From<TxEip1559> for EthereumTypedTransaction<Eip4844> {
    fn from(tx: TxEip1559) -> Self {
        Self::Eip1559(tx)
    }
}

impl<Eip4844: From<TxEip4844>> From<TxEip4844> for EthereumTypedTransaction<Eip4844> {
    fn from(tx: TxEip4844) -> Self {
        Self::Eip4844(tx.into())
    }
}

impl<Eip4844: From<TxEip4844WithSidecar>> From<TxEip4844WithSidecar>
    for EthereumTypedTransaction<Eip4844>
{
    fn from(tx: TxEip4844WithSidecar) -> Self {
        Self::Eip4844(tx.into())
    }
}

impl<Eip4844: From<TxEip4844Variant>> From<TxEip4844Variant> for EthereumTypedTransaction<Eip4844> {
    fn from(tx: TxEip4844Variant) -> Self {
        Self::Eip4844(tx.into())
    }
}

impl<Eip4844> From<TxEip7702> for EthereumTypedTransaction<Eip4844> {
    fn from(tx: TxEip7702) -> Self {
        Self::Eip7702(tx)
    }
}

impl<Eip4844> From<EthereumTxEnvelope<Eip4844>> for EthereumTypedTransaction<Eip4844> {
    fn from(envelope: EthereumTxEnvelope<Eip4844>) -> Self {
        match envelope {
            EthereumTxEnvelope::Legacy(tx) => Self::Legacy(tx.strip_signature()),
            EthereumTxEnvelope::Eip2930(tx) => Self::Eip2930(tx.strip_signature()),
            EthereumTxEnvelope::Eip1559(tx) => Self::Eip1559(tx.strip_signature()),
            EthereumTxEnvelope::Eip4844(tx) => Self::Eip4844(tx.strip_signature()),
            EthereumTxEnvelope::Eip7702(tx) => Self::Eip7702(tx.strip_signature()),
        }
    }
}

impl<Eip4844: RlpEcdsaEncodableTx> EthereumTypedTransaction<Eip4844> {
    /// Return the [`TxType`] of the inner txn.
    #[doc(alias = "transaction_type")]
    pub const fn tx_type(&self) -> TxType {
        match self {
            Self::Legacy(_) => TxType::Legacy,
            Self::Eip2930(_) => TxType::Eip2930,
            Self::Eip1559(_) => TxType::Eip1559,
            Self::Eip4844(_) => TxType::Eip4844,
            Self::Eip7702(_) => TxType::Eip7702,
        }
    }

    /// Return the inner legacy transaction if it exists.
    pub const fn legacy(&self) -> Option<&TxLegacy> {
        match self {
            Self::Legacy(tx) => Some(tx),
            _ => None,
        }
    }

    /// Return the inner EIP-2930 transaction if it exists.
    pub const fn eip2930(&self) -> Option<&TxEip2930> {
        match self {
            Self::Eip2930(tx) => Some(tx),
            _ => None,
        }
    }

    /// Return the inner EIP-1559 transaction if it exists.
    pub const fn eip1559(&self) -> Option<&TxEip1559> {
        match self {
            Self::Eip1559(tx) => Some(tx),
            _ => None,
        }
    }

    /// Return the inner EIP-7702 transaction if it exists.
    pub const fn eip7702(&self) -> Option<&TxEip7702> {
        match self {
            Self::Eip7702(tx) => Some(tx),
            _ => None,
        }
    }

    /// Calculate the transaction hash for the given signature.
    pub fn tx_hash(&self, signature: &Signature) -> TxHash {
        match self {
            Self::Legacy(tx) => tx.tx_hash(signature),
            Self::Eip2930(tx) => tx.tx_hash(signature),
            Self::Eip1559(tx) => tx.tx_hash(signature),
            Self::Eip4844(tx) => tx.tx_hash(signature),
            Self::Eip7702(tx) => tx.tx_hash(signature),
        }
    }
}

impl<Eip4844: Transaction> Transaction for EthereumTypedTransaction<Eip4844> {
    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        match self {
            Self::Legacy(tx) => tx.chain_id(),
            Self::Eip2930(tx) => tx.chain_id(),
            Self::Eip1559(tx) => tx.chain_id(),
            Self::Eip4844(tx) => tx.chain_id(),
            Self::Eip7702(tx) => tx.chain_id(),
        }
    }

    #[inline]
    fn nonce(&self) -> u64 {
        match self {
            Self::Legacy(tx) => tx.nonce(),
            Self::Eip2930(tx) => tx.nonce(),
            Self::Eip1559(tx) => tx.nonce(),
            Self::Eip4844(tx) => tx.nonce(),
            Self::Eip7702(tx) => tx.nonce(),
        }
    }

    #[inline]
    fn gas_limit(&self) -> u64 {
        match self {
            Self::Legacy(tx) => tx.gas_limit(),
            Self::Eip2930(tx) => tx.gas_limit(),
            Self::Eip1559(tx) => tx.gas_limit(),
            Self::Eip4844(tx) => tx.gas_limit(),
            Self::Eip7702(tx) => tx.gas_limit(),
        }
    }

    #[inline]
    fn gas_price(&self) -> Option<u128> {
        match self {
            Self::Legacy(tx) => tx.gas_price(),
            Self::Eip2930(tx) => tx.gas_price(),
            Self::Eip1559(tx) => tx.gas_price(),
            Self::Eip4844(tx) => tx.gas_price(),
            Self::Eip7702(tx) => tx.gas_price(),
        }
    }

    #[inline]
    fn max_fee_per_gas(&self) -> u128 {
        match self {
            Self::Legacy(tx) => tx.max_fee_per_gas(),
            Self::Eip2930(tx) => tx.max_fee_per_gas(),
            Self::Eip1559(tx) => tx.max_fee_per_gas(),
            Self::Eip4844(tx) => tx.max_fee_per_gas(),
            Self::Eip7702(tx) => tx.max_fee_per_gas(),
        }
    }

    #[inline]
    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        match self {
            Self::Legacy(tx) => tx.max_priority_fee_per_gas(),
            Self::Eip2930(tx) => tx.max_priority_fee_per_gas(),
            Self::Eip1559(tx) => tx.max_priority_fee_per_gas(),
            Self::Eip4844(tx) => tx.max_priority_fee_per_gas(),
            Self::Eip7702(tx) => tx.max_priority_fee_per_gas(),
        }
    }

    #[inline]
    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        match self {
            Self::Legacy(tx) => tx.max_fee_per_blob_gas(),
            Self::Eip2930(tx) => tx.max_fee_per_blob_gas(),
            Self::Eip1559(tx) => tx.max_fee_per_blob_gas(),
            Self::Eip4844(tx) => tx.max_fee_per_blob_gas(),
            Self::Eip7702(tx) => tx.max_fee_per_blob_gas(),
        }
    }

    #[inline]
    fn priority_fee_or_price(&self) -> u128 {
        match self {
            Self::Legacy(tx) => tx.priority_fee_or_price(),
            Self::Eip2930(tx) => tx.priority_fee_or_price(),
            Self::Eip1559(tx) => tx.priority_fee_or_price(),
            Self::Eip4844(tx) => tx.priority_fee_or_price(),
            Self::Eip7702(tx) => tx.priority_fee_or_price(),
        }
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        match self {
            Self::Legacy(tx) => tx.effective_gas_price(base_fee),
            Self::Eip2930(tx) => tx.effective_gas_price(base_fee),
            Self::Eip1559(tx) => tx.effective_gas_price(base_fee),
            Self::Eip4844(tx) => tx.effective_gas_price(base_fee),
            Self::Eip7702(tx) => tx.effective_gas_price(base_fee),
        }
    }

    #[inline]
    fn is_dynamic_fee(&self) -> bool {
        match self {
            Self::Legacy(tx) => tx.is_dynamic_fee(),
            Self::Eip2930(tx) => tx.is_dynamic_fee(),
            Self::Eip1559(tx) => tx.is_dynamic_fee(),
            Self::Eip4844(tx) => tx.is_dynamic_fee(),
            Self::Eip7702(tx) => tx.is_dynamic_fee(),
        }
    }

    #[inline]
    fn kind(&self) -> TxKind {
        match self {
            Self::Legacy(tx) => tx.kind(),
            Self::Eip2930(tx) => tx.kind(),
            Self::Eip1559(tx) => tx.kind(),
            Self::Eip4844(tx) => tx.kind(),
            Self::Eip7702(tx) => tx.kind(),
        }
    }

    #[inline]
    fn is_create(&self) -> bool {
        match self {
            Self::Legacy(tx) => tx.is_create(),
            Self::Eip2930(tx) => tx.is_create(),
            Self::Eip1559(tx) => tx.is_create(),
            Self::Eip4844(tx) => tx.is_create(),
            Self::Eip7702(tx) => tx.is_create(),
        }
    }

    #[inline]
    fn value(&self) -> U256 {
        match self {
            Self::Legacy(tx) => tx.value(),
            Self::Eip2930(tx) => tx.value(),
            Self::Eip1559(tx) => tx.value(),
            Self::Eip4844(tx) => tx.value(),
            Self::Eip7702(tx) => tx.value(),
        }
    }

    #[inline]
    fn input(&self) -> &Bytes {
        match self {
            Self::Legacy(tx) => tx.input(),
            Self::Eip2930(tx) => tx.input(),
            Self::Eip1559(tx) => tx.input(),
            Self::Eip4844(tx) => tx.input(),
            Self::Eip7702(tx) => tx.input(),
        }
    }

    #[inline]
    fn access_list(&self) -> Option<&AccessList> {
        match self {
            Self::Legacy(tx) => tx.access_list(),
            Self::Eip2930(tx) => tx.access_list(),
            Self::Eip1559(tx) => tx.access_list(),
            Self::Eip4844(tx) => tx.access_list(),
            Self::Eip7702(tx) => tx.access_list(),
        }
    }

    #[inline]
    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        match self {
            Self::Legacy(tx) => tx.blob_versioned_hashes(),
            Self::Eip2930(tx) => tx.blob_versioned_hashes(),
            Self::Eip1559(tx) => tx.blob_versioned_hashes(),
            Self::Eip4844(tx) => tx.blob_versioned_hashes(),
            Self::Eip7702(tx) => tx.blob_versioned_hashes(),
        }
    }

    #[inline]
    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        match self {
            Self::Legacy(tx) => tx.authorization_list(),
            Self::Eip2930(tx) => tx.authorization_list(),
            Self::Eip1559(tx) => tx.authorization_list(),
            Self::Eip4844(tx) => tx.authorization_list(),
            Self::Eip7702(tx) => tx.authorization_list(),
        }
    }
}

impl<Eip4844: Typed2718> Typed2718 for EthereumTypedTransaction<Eip4844> {
    fn ty(&self) -> u8 {
        match self {
            Self::Legacy(tx) => tx.ty(),
            Self::Eip2930(tx) => tx.ty(),
            Self::Eip1559(tx) => tx.ty(),
            Self::Eip4844(tx) => tx.ty(),
            Self::Eip7702(tx) => tx.ty(),
        }
    }
}

impl<Eip4844: RlpEcdsaEncodableTx> RlpEcdsaEncodableTx for EthereumTypedTransaction<Eip4844> {
    const DEFAULT_TX_TYPE: u8 = 0;

    fn rlp_encoded_fields_length(&self) -> usize {
        match self {
            Self::Legacy(tx) => tx.rlp_encoded_fields_length(),
            Self::Eip2930(tx) => tx.rlp_encoded_fields_length(),
            Self::Eip1559(tx) => tx.rlp_encoded_fields_length(),
            Self::Eip4844(tx) => tx.rlp_encoded_fields_length(),
            Self::Eip7702(tx) => tx.rlp_encoded_fields_length(),
        }
    }

    fn rlp_encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            Self::Legacy(tx) => tx.rlp_encode_fields(out),
            Self::Eip2930(tx) => tx.rlp_encode_fields(out),
            Self::Eip1559(tx) => tx.rlp_encode_fields(out),
            Self::Eip4844(tx) => tx.rlp_encode_fields(out),
            Self::Eip7702(tx) => tx.rlp_encode_fields(out),
        }
    }

    fn eip2718_encode_with_type(&self, signature: &Signature, _ty: u8, out: &mut dyn BufMut) {
        match self {
            Self::Legacy(tx) => tx.eip2718_encode_with_type(signature, tx.ty(), out),
            Self::Eip2930(tx) => tx.eip2718_encode_with_type(signature, tx.ty(), out),
            Self::Eip1559(tx) => tx.eip2718_encode_with_type(signature, tx.ty(), out),
            Self::Eip4844(tx) => tx.eip2718_encode_with_type(signature, tx.ty(), out),
            Self::Eip7702(tx) => tx.eip2718_encode_with_type(signature, tx.ty(), out),
        }
    }

    fn eip2718_encode(&self, signature: &Signature, out: &mut dyn BufMut) {
        match self {
            Self::Legacy(tx) => tx.eip2718_encode(signature, out),
            Self::Eip2930(tx) => tx.eip2718_encode(signature, out),
            Self::Eip1559(tx) => tx.eip2718_encode(signature, out),
            Self::Eip4844(tx) => tx.eip2718_encode(signature, out),
            Self::Eip7702(tx) => tx.eip2718_encode(signature, out),
        }
    }

    fn network_encode_with_type(&self, signature: &Signature, _ty: u8, out: &mut dyn BufMut) {
        match self {
            Self::Legacy(tx) => tx.network_encode_with_type(signature, tx.ty(), out),
            Self::Eip2930(tx) => tx.network_encode_with_type(signature, tx.ty(), out),
            Self::Eip1559(tx) => tx.network_encode_with_type(signature, tx.ty(), out),
            Self::Eip4844(tx) => tx.network_encode_with_type(signature, tx.ty(), out),
            Self::Eip7702(tx) => tx.network_encode_with_type(signature, tx.ty(), out),
        }
    }

    fn network_encode(&self, signature: &Signature, out: &mut dyn BufMut) {
        match self {
            Self::Legacy(tx) => tx.network_encode(signature, out),
            Self::Eip2930(tx) => tx.network_encode(signature, out),
            Self::Eip1559(tx) => tx.network_encode(signature, out),
            Self::Eip4844(tx) => tx.network_encode(signature, out),
            Self::Eip7702(tx) => tx.network_encode(signature, out),
        }
    }

    fn tx_hash_with_type(&self, signature: &Signature, _ty: u8) -> TxHash {
        match self {
            Self::Legacy(tx) => tx.tx_hash_with_type(signature, tx.ty()),
            Self::Eip2930(tx) => tx.tx_hash_with_type(signature, tx.ty()),
            Self::Eip1559(tx) => tx.tx_hash_with_type(signature, tx.ty()),
            Self::Eip4844(tx) => tx.tx_hash_with_type(signature, tx.ty()),
            Self::Eip7702(tx) => tx.tx_hash_with_type(signature, tx.ty()),
        }
    }

    fn tx_hash(&self, signature: &Signature) -> TxHash {
        match self {
            Self::Legacy(tx) => tx.tx_hash(signature),
            Self::Eip2930(tx) => tx.tx_hash(signature),
            Self::Eip1559(tx) => tx.tx_hash(signature),
            Self::Eip4844(tx) => tx.tx_hash(signature),
            Self::Eip7702(tx) => tx.tx_hash(signature),
        }
    }
}

impl<Eip4844: SignableTransaction<Signature>> SignableTransaction<Signature>
    for EthereumTypedTransaction<Eip4844>
{
    fn set_chain_id(&mut self, chain_id: ChainId) {
        match self {
            Self::Legacy(tx) => tx.set_chain_id(chain_id),
            Self::Eip2930(tx) => tx.set_chain_id(chain_id),
            Self::Eip1559(tx) => tx.set_chain_id(chain_id),
            Self::Eip4844(tx) => tx.set_chain_id(chain_id),
            Self::Eip7702(tx) => tx.set_chain_id(chain_id),
        }
    }

    fn encode_for_signing(&self, out: &mut dyn BufMut) {
        match self {
            Self::Legacy(tx) => tx.encode_for_signing(out),
            Self::Eip2930(tx) => tx.encode_for_signing(out),
            Self::Eip1559(tx) => tx.encode_for_signing(out),
            Self::Eip4844(tx) => tx.encode_for_signing(out),
            Self::Eip7702(tx) => tx.encode_for_signing(out),
        }
    }

    fn payload_len_for_signature(&self) -> usize {
        match self {
            Self::Legacy(tx) => tx.payload_len_for_signature(),
            Self::Eip2930(tx) => tx.payload_len_for_signature(),
            Self::Eip1559(tx) => tx.payload_len_for_signature(),
            Self::Eip4844(tx) => tx.payload_len_for_signature(),
            Self::Eip7702(tx) => tx.payload_len_for_signature(),
        }
    }
}

#[cfg(feature = "serde")]
impl<Eip4844, T: From<EthereumTypedTransaction<Eip4844>>> From<EthereumTypedTransaction<Eip4844>>
    for alloy_serde::WithOtherFields<T>
{
    fn from(value: EthereumTypedTransaction<Eip4844>) -> Self {
        Self::new(value.into())
    }
}

#[cfg(feature = "serde")]
impl<Eip4844, T> From<EthereumTxEnvelope<Eip4844>> for alloy_serde::WithOtherFields<T>
where
    T: From<EthereumTxEnvelope<Eip4844>>,
{
    fn from(value: EthereumTxEnvelope<Eip4844>) -> Self {
        Self::new(value.into())
    }
}

#[cfg(feature = "serde")]
mod serde_from {
    //! NB: Why do we need this?
    //!
    //! Because the tag may be missing, we need an abstraction over tagged (with
    //! type) and untagged (always legacy). This is
    //! [`MaybeTaggedTypedTransaction`].
    //!
    //! The tagged variant is [`TaggedTypedTransaction`], which always has a
    //! type tag.
    //!
    //! We serialize via [`TaggedTypedTransaction`] and deserialize via
    //! [`MaybeTaggedTypedTransaction`].
    use crate::{EthereumTypedTransaction, TxEip1559, TxEip2930, TxEip7702, TxLegacy};

    #[derive(Debug, serde::Deserialize)]
    #[serde(untagged)]
    pub(crate) enum MaybeTaggedTypedTransaction<Eip4844> {
        Tagged(TaggedTypedTransaction<Eip4844>),
        Untagged {
            #[serde(default, rename = "type", deserialize_with = "alloy_serde::reject_if_some")]
            _ty: Option<()>,
            #[serde(flatten)]
            tx: TxLegacy,
        },
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    #[serde(tag = "type")]
    pub(crate) enum TaggedTypedTransaction<Eip4844> {
        /// Legacy transaction
        #[serde(rename = "0x00", alias = "0x0")]
        Legacy(TxLegacy),
        /// EIP-2930 transaction
        #[serde(rename = "0x01", alias = "0x1")]
        Eip2930(TxEip2930),
        /// EIP-1559 transaction
        #[serde(rename = "0x02", alias = "0x2")]
        Eip1559(TxEip1559),
        /// EIP-4844 transaction
        #[serde(rename = "0x03", alias = "0x3")]
        Eip4844(Eip4844),
        /// EIP-7702 transaction
        #[serde(rename = "0x04", alias = "0x4")]
        Eip7702(TxEip7702),
    }

    impl<Eip4844> From<MaybeTaggedTypedTransaction<Eip4844>> for EthereumTypedTransaction<Eip4844> {
        fn from(value: MaybeTaggedTypedTransaction<Eip4844>) -> Self {
            match value {
                MaybeTaggedTypedTransaction::Tagged(tagged) => tagged.into(),
                MaybeTaggedTypedTransaction::Untagged { tx, .. } => Self::Legacy(tx),
            }
        }
    }

    impl<Eip4844> From<TaggedTypedTransaction<Eip4844>> for EthereumTypedTransaction<Eip4844> {
        fn from(value: TaggedTypedTransaction<Eip4844>) -> Self {
            match value {
                TaggedTypedTransaction::Legacy(signed) => Self::Legacy(signed),
                TaggedTypedTransaction::Eip2930(signed) => Self::Eip2930(signed),
                TaggedTypedTransaction::Eip1559(signed) => Self::Eip1559(signed),
                TaggedTypedTransaction::Eip4844(signed) => Self::Eip4844(signed),
                TaggedTypedTransaction::Eip7702(signed) => Self::Eip7702(signed),
            }
        }
    }

    impl<Eip4844> From<EthereumTypedTransaction<Eip4844>> for TaggedTypedTransaction<Eip4844> {
        fn from(value: EthereumTypedTransaction<Eip4844>) -> Self {
            match value {
                EthereumTypedTransaction::Legacy(signed) => Self::Legacy(signed),
                EthereumTypedTransaction::Eip2930(signed) => Self::Eip2930(signed),
                EthereumTypedTransaction::Eip1559(signed) => Self::Eip1559(signed),
                EthereumTypedTransaction::Eip4844(signed) => Self::Eip4844(signed),
                EthereumTypedTransaction::Eip7702(signed) => Self::Eip7702(signed),
            }
        }
    }
}
