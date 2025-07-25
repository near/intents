use bip322::verify_simple;
use bitcoin::{
    Address, Amount, EcdsaSighashType, Psbt, Script, ScriptBuf, Sequence, Transaction, TxIn, TxOut,
    Txid, Witness, WitnessVersion, absolute,
    address::{AddressData, NetworkUnchecked},
    consensus::Encodable,
    hashes::Hash,
    opcodes, script,
    sighash::SighashCache,
    transaction::{OutPoint, Version},
};
use defuse_bip340::{Bip340TaggedDigest, Double};
use defuse_crypto::{Curve, Payload, Secp256k1, SignedPayload, serde::AsCurve};
use defuse_near_utils::digest::Sha256;
use digest::Digest;
use near_sdk::near;
use serde_with::serde_as;

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[serde(rename_all = "snake_case")]
#[derive(Debug, Clone)]
/// [BIP-322](https://github.com/bitcoin/bips/blob/master/bip-0322.mediawiki)
pub struct SignedBip322Payload {
    pub address: Address<
        // TODO
        NetworkUnchecked,
    >,
    pub message: String,

    /// BIP-322 signature data as a witness stack.
    /// 
    /// The witness format depends on the address type:
    /// - P2PKH/P2WPKH: [signature, pubkey]
    /// - P2SH: [signature, pubkey, redeem_script]
    /// - P2WSH: [signature, pubkey, witness_script]
    pub signature: Witness,
    // #[serde_as(as = "AsCurve<Secp256k1>")]
    // pub signature: <Secp256k1 as Curve>::Signature,
}

impl Payload for SignedBip322Payload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        match self
            .address
            .assume_checked_ref()
            .to_address_data()
        {
            AddressData::P2pkh { pubkey_hash } => todo!(),
            AddressData::P2sh { script_hash } => todo!(),
            // P2WPKH
            AddressData::Segwit { witness_program } if witness_program.is_p2wpkh() => {
                todo!()
            }
            // P2WSH
            AddressData::Segwit { witness_program } if witness_program.is_p2wsh() => {
                todo!()
            }

            _ => todo!(),
        }
    }
}

impl SignedPayload for SignedBip322Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        // TODO: references:
        // * https://github.com/ACken2/bip322-js/blob/7c30636fe0be968c52527266544296c535ab0936/src/Verifier.ts#L24
        // * https://github.com/rust-bitcoin/bip322/blob/f6e4f4d87cc6bdf07a1dc937d92e10f1d9ceaef4/src/verify.rs#L60-L94

        let address = self
            .address
            // TODO
            .assume_checked_ref();

        let to_spend = create_to_spend(&address, &self.message);
        let to_sign = create_to_sign(&to_spend);

        let script_code = match address.to_address_data() {
            AddressData::P2pkh { pubkey_hash } => {
                &to_spend
                    .output
                    .first()
                    // TODO
                    .unwrap()
                    .script_pubkey
            }
            AddressData::P2sh { script_hash } => {
                let script = to_spend
                    .input
                    .first()
                    // TODO
                    .unwrap()
                    .script_sig;
                let instructions = script.instructions_minimal();
                instructions.next()?.ok()?;
                todo!()
                // script.to_owned().redeem_script().unwrap().is_p2wpkh()
            }
            // P2WPKH
            AddressData::Segwit { witness_program } if witness_program.is_p2wpkh() => {
                &to_spend
                    .output
                    .first()
                    // TODO
                    .unwrap()
                    .script_pubkey
            }
            // P2WSH
            AddressData::Segwit { witness_program } if witness_program.is_p2wsh() => {
                todo!()
            }
            // P2TR (Pay-to-Taproot) is not supported (cannot recover public key?)
            _ => todo!(),
        };

        let sighash = {
            let mut sighash_cache = SighashCache::new(to_sign);
            let mut buf = Vec::new();
            sighash_cache.segwit_v0_encode_signing_data_to(
                &mut buf,
                0,
                script_code,
                to_spend
                    .output
                    .first()
                    // TODO
                    .unwrap()
                    .value,
                EcdsaSighashType::All,
            );
            Double::<Sha256>::digest(buf).into()
        };

        // TODO: recovery byte is not in the siganture, but it might be possible to recoonstruct it:
        // https://bitcoin.stackexchange.com/questions/83035/how-to-determine-first-byte-recovery-id-for-signatures-message-signing
        Secp256k1::verify(todo!(), &sighash, &());

        todo!()

        // sighash_cache.p2wsh_signature_hash(input_index, witness_script, value, sighash_type)

        // verify_simple(&self.address, message, self.signature)
        //     .ok()
        //     .map(|()| self.address.assume_checked_ref().script_pubkey())
        // Secp256k1::verify(&self.signature, &self.hash(), &())
    }
}
const BIP322_TAG: &[u8] = b"BIP0322-signed-message";

fn create_to_spend(address: &Address, message: impl AsRef<[u8]>) -> Transaction {
    Transaction {
        version: Version(0),
        lock_time: absolute::LockTime::ZERO,
        input: [TxIn {
            previous_output: OutPoint::new(Txid::all_zeros(), 0xFFFFFFFF),
            script_sig: script::Builder::new()
                .push_opcode(opcodes::OP_0)
                .push_slice(<[u8; 32]>::from(
                    Sha256::tagged(BIP322_TAG).chain_update(message).finalize(),
                ))
                .into_script(),
            sequence: Sequence::ZERO,
            witness: Witness::new(),
        }]
        .into(),
        output: [TxOut {
            value: Amount::ZERO,
            script_pubkey: address.script_pubkey(),
        }]
        .into(),
    }
}

fn create_to_sign(to_spend: &Transaction) -> Transaction {
    Transaction {
        version: Version(0),
        lock_time: absolute::LockTime::ZERO,
        input: [TxIn {
            previous_output: OutPoint::new(Txid::from_byte_array(tx_id(to_spend)), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ZERO,
            witness: Witness::new(), // TODO
        }]
        .into(),
        output: [TxOut {
            value: Amount::ZERO,
            script_pubkey: script::Builder::new()
                .push_opcode(opcodes::all::OP_RETURN)
                .into_script(),
        }]
        .into(),
    }
}

fn tx_id(tx: &Transaction) -> [u8; 32] {
    // TODO
    // tx.compute_txid().to_raw_hash().to_byte_array()
    let mut buf = Vec::new();
    tx.consensus_encode(&mut buf)
        .unwrap_or_else(|_| unreachable!());
    Double::<Sha256>::digest(buf).into()
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        b"",
        hex!("c90c269c4f8fcbe6880f72a721ddfbf1914268a794cbb21cfafee13770ae19f1"),
    )]
    #[case(
        b"Hello World",
        hex!("f0eb03b1a75ac6d9847f55c624a99169b5dccba2a31f5b23bea77ba270de0a7a"),
    )]
    fn message_hash(#[case] message: &[u8], #[case] hash: [u8; 32]) {
        assert_eq!(
            Sha256::tagged(BIP322_TAG).chain_update(message).finalize(),
            hash.into(),
        );
    }

    // TODO
    #[rstest]
    #[case(
        "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l".parse().unwrap(),
        b"",
        hex!("c5680aa69bb8d860bf82d4e9cd3504b55dde018de765a91bb566283c545a99a7"),
        hex!("1e9654e951a5ba44c8604c4de6c67fd78a27e81dcadcfe1edf638ba3aaebaed6"),
    )]
    #[case(
        "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l".parse().unwrap(),
        b"Hello World",
        hex!("b79d196740ad5217771c1098fc4a4b51e0535c32236c71f1ea4d61a2d603352b"),
        hex!("88737ae86f2077145f93cc4b153ae9a1cb8d56afa511988c149c5c8c9d93bddf"),
    )]
    fn transaction_hash(
        #[case] address: Address<NetworkUnchecked>,
        #[case] message: &[u8],
        #[case] to_spend_hash: [u8; 32],
        #[case] to_sign_hash: [u8; 32],
    ) {
        let to_spend = create_to_spend(address.assume_checked_ref(), message);
        assert_eq!(tx_id(&to_spend), to_spend_hash, "to_spend");

        let to_sign = create_to_sign(&to_spend);
        assert_eq!(tx_id(&to_sign), to_sign_hash, "to_sign");
    }
}
