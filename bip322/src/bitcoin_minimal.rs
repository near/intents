// Minimal Bitcoin types for BIP-322 implementation
// Only includes what's needed for P2PKH/P2WPKH address handling

use near_sdk::{near, env};
use serde_with::serde_as;

/// Minimal Bitcoin address representation for BIP-322
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Address {
    pub inner: String,
    pub address_type: AddressType,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub enum AddressType {
    P2PKH,
    P2WPKH,
    // Phase 4: P2SH, P2WSH  
}

#[derive(Debug, Clone)]
pub enum AddressData {
    P2pkh { pubkey_hash: [u8; 20] },
    Segwit { witness_program: WitnessProgram },
    P2sh { script_hash: [u8; 20] },
}

#[derive(Debug, Clone)]
pub struct WitnessProgram {
    version: u8,
    program: Vec<u8>,
}

impl WitnessProgram {
    pub fn is_p2wpkh(&self) -> bool {
        self.version == 0 && self.program.len() == 20
    }
    
    pub fn is_p2wsh(&self) -> bool {
        self.version == 0 && self.program.len() == 32
    }
}

/// Minimal Witness implementation
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Witness {
    stack: Vec<Vec<u8>>,
}

impl Witness {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }
    
    pub fn len(&self) -> usize {
        self.stack.len()
    }
    
    pub fn nth(&self, index: usize) -> Option<&[u8]> {
        self.stack.get(index).map(|v| v.as_slice())
    }
}

impl Address {
    pub fn assume_checked_ref(&self) -> &Self {
        self
    }
    
    pub fn to_address_data(&self) -> AddressData {
        match self.address_type {
            AddressType::P2PKH => {
                // For P2PKH, extract pubkey hash from address
                // Simplified: just use a placeholder hash for now
                AddressData::P2pkh { 
                    pubkey_hash: [0u8; 20] // TODO: Parse from address string
                }
            },
            AddressType::P2WPKH => {
                // For P2WPKH, create witness program
                AddressData::Segwit {
                    witness_program: WitnessProgram {
                        version: 0,
                        program: vec![0u8; 20], // TODO: Parse from address string
                    }
                }
            },
        }
    }
    
    pub fn script_pubkey(&self) -> ScriptBuf {
        match self.address_type {
            AddressType::P2PKH => {
                // P2PKH script: OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG
                let mut script = Vec::new();
                script.push(0x76); // OP_DUP
                script.push(0xa9); // OP_HASH160
                script.push(20);   // Push 20 bytes
                script.extend_from_slice(&[0u8; 20]); // TODO: actual pubkey hash
                script.push(0x88); // OP_EQUALVERIFY
                script.push(0xac); // OP_CHECKSIG
                ScriptBuf { inner: script }
            },
            AddressType::P2WPKH => {
                // P2WPKH script: OP_0 <20-byte-pubkey-hash>
                let mut script = Vec::new();
                script.push(0x00); // OP_0
                script.push(20);   // Push 20 bytes
                script.extend_from_slice(&[0u8; 20]); // TODO: actual pubkey hash
                ScriptBuf { inner: script }
            },
        }
    }
}

impl std::str::FromStr for Address {
    type Err = &'static str;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Simplified address parsing for MVP
        if s.starts_with('1') {
            Ok(Address {
                inner: s.to_string(),
                address_type: AddressType::P2PKH,
            })
        } else if s.starts_with("bc1q") {
            Ok(Address {
                inner: s.to_string(),
                address_type: AddressType::P2WPKH,
            })
        } else {
            Err("Unsupported address format")
        }
    }
}

/// Script buffer
#[derive(Debug, Clone)]
pub struct ScriptBuf {
    inner: Vec<u8>,
}

impl ScriptBuf {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }
}

/// Transaction ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Txid([u8; 32]);

impl Txid {
    pub fn all_zeros() -> Self {
        Self([0u8; 32])
    }
    
    pub fn from_byte_array(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Transaction output point
#[derive(Debug, Clone)]
pub struct OutPoint {
    pub txid: Txid,
    pub vout: u32,
}

impl OutPoint {
    pub fn new(txid: Txid, vout: u32) -> Self {
        Self { txid, vout }
    }
}

/// Transaction input
#[derive(Debug, Clone)]
pub struct TxIn {
    pub previous_output: OutPoint,
    pub script_sig: ScriptBuf,
    pub sequence: Sequence,
    pub witness: Witness,
}

/// Transaction output  
#[derive(Debug, Clone)]
pub struct TxOut {
    pub value: Amount,
    pub script_pubkey: ScriptBuf,
}

/// Transaction
#[derive(Debug, Clone)]
pub struct Transaction {
    pub version: Version,
    pub lock_time: LockTime,
    pub input: Vec<TxIn>,
    pub output: Vec<TxOut>,
}

/// Amount (simplified)
#[derive(Debug, Clone, Copy)]
pub struct Amount(u64);

impl Amount {
    pub const ZERO: Self = Self(0);
}

/// Version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version(pub i32);

/// Lock time
#[derive(Debug, Clone, Copy)]
pub struct LockTime(u32);

impl LockTime {
    pub const ZERO: Self = Self(0);
}

/// Sequence
#[derive(Debug, Clone, Copy)]
pub struct Sequence(u32);

impl Sequence {
    pub const ZERO: Self = Self(0);
}

/// Consensus encodable trait
pub trait Encodable {
    fn consensus_encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error>;
}

impl Encodable for Transaction {
    fn consensus_encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let mut len = 0;
        
        // Version (4 bytes, little-endian)
        len += writer.write(&self.version.0.to_le_bytes())?;
        
        // Input count (compact size)
        len += write_compact_size(writer, self.input.len() as u64)?;
        
        // Inputs
        for input in &self.input {
            // Previous output (36 bytes)
            len += writer.write(&input.previous_output.txid.0)?;
            len += writer.write(&input.previous_output.vout.to_le_bytes())?;
            
            // Script sig
            len += write_compact_size(writer, input.script_sig.inner.len() as u64)?;
            len += writer.write(&input.script_sig.inner)?;
            
            // Sequence (4 bytes)
            len += writer.write(&input.sequence.0.to_le_bytes())?;
        }
        
        // Output count
        len += write_compact_size(writer, self.output.len() as u64)?;
        
        // Outputs
        for output in &self.output {
            // Value (8 bytes, little-endian)
            len += writer.write(&output.value.0.to_le_bytes())?;
            
            // Script pubkey
            len += write_compact_size(writer, output.script_pubkey.inner.len() as u64)?;
            len += writer.write(&output.script_pubkey.inner)?;
        }
        
        // Lock time (4 bytes)
        len += writer.write(&self.lock_time.0.to_le_bytes())?;
        
        Ok(len)
    }
}

fn write_compact_size<W: std::io::Write>(writer: &mut W, n: u64) -> Result<usize, std::io::Error> {
    if n < 0xfd {
        writer.write(&[n as u8])?;
        Ok(1)
    } else if n <= 0xffff {
        writer.write(&[0xfd])?;
        writer.write(&(n as u16).to_le_bytes())?;
        Ok(3)
    } else if n <= 0xffffffff {
        writer.write(&[0xfe])?;
        writer.write(&(n as u32).to_le_bytes())?;
        Ok(5)
    } else {
        writer.write(&[0xff])?;
        writer.write(&n.to_le_bytes())?;
        Ok(9)
    }
}

/// Script builder
pub struct ScriptBuilder {
    inner: Vec<u8>,
}

impl ScriptBuilder {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }
    
    pub fn push_opcode(mut self, opcode: u8) -> Self {
        self.inner.push(opcode);
        self
    }
    
    pub fn push_slice(mut self, data: &[u8]) -> Self {
        if data.len() <= 75 {
            self.inner.push(data.len() as u8);
        } else {
            panic!("Large pushdata not implemented");
        }
        self.inner.extend_from_slice(data);
        self
    }
    
    pub fn into_script(self) -> ScriptBuf {
        ScriptBuf { inner: self.inner }
    }
}

// Op codes  
pub const OP_0: u8 = 0x00;
pub const OP_RETURN: u8 = 0x6a;

// Signature hash cache (simplified)
pub struct SighashCache {
    tx: Transaction,
}

impl SighashCache {
    pub fn new(tx: Transaction) -> Self {
        Self { tx }
    }
    
    pub fn segwit_v0_encode_signing_data_to<W: std::io::Write>(
        &mut self,
        writer: &mut W,
        input_index: usize,
        script_code: &ScriptBuf,
        value: Amount,
        sighash_type: EcdsaSighashType,
    ) -> Result<(), std::io::Error> {
        // Simplified segwit v0 sighash implementation
        // This is a placeholder - full implementation would be more complex
        
        // For MVP, just write some basic transaction data
        writer.write(&self.tx.version.0.to_le_bytes())?;
        writer.write(&[input_index as u8])?;
        writer.write(&script_code.inner)?;
        writer.write(&value.0.to_le_bytes())?;
        writer.write(&[sighash_type as u8])?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EcdsaSighashType {
    All = 0x01,
}