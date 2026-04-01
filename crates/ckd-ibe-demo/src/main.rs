use blstrs::{G1Affine, G1Projective, G2Affine, G2Projective, Gt, Scalar};
use elliptic_curve::{Field as _, Group as _};
use hkdf::Hkdf;
use rand_core::OsRng;
use sha2::Sha256;
use sha3::{Digest, Sha3_256};

const NEAR_CKD_DOMAIN: &[u8] = b"NEAR BLS12381G1_XMD:SHA-256_SSWU_RO_";

// =============================================================================
// Shared types
// =============================================================================

/// Public parameters known to everyone: the MPC master public key and the app identity.
pub struct PublicParams {
    pub pk_mpc: G2Projective,
    pub app_id: Vec<u8>,
}

/// IBE ciphertext: the ephemeral G2 point + XOR-encrypted payload.
pub struct Ciphertext {
    /// U = r·G2  (ephemeral public value sent alongside the ciphertext)
    pub u: G2Projective,
    /// XOR-encrypted payload
    pub bytes: Vec<u8>,
}

// =============================================================================
// Shared crypto utilities
// =============================================================================

fn hash_to_g1(input: &[u8]) -> G1Projective {
    G1Projective::hash_to_curve(input, NEAR_CKD_DOMAIN, &[])
}

fn hash_app_id_with_pk(pk_mpc: &G2Projective, app_id: &[u8]) -> G1Projective {
    let pk_bytes = G2Affine::from(pk_mpc).to_compressed();
    let input = [pk_bytes.as_slice(), app_id].concat();
    hash_to_g1(&input)
}

/// Derive app_id from account_id and derivation_path (same algorithm as the contract).
pub fn derive_app_id(account_id: &str, path: &str) -> Vec<u8> {
    let prefix = "near-mpc v0.1.0 app_id derivation:";
    let input = format!("{prefix}{account_id},{path}");
    let mut hasher = Sha3_256::new();
    hasher.update(input.as_bytes());
    hasher.finalize().to_vec()
}

/// Derive a 32-byte symmetric key from a GT element.
fn symmetric_key_from_gt(gt: &Gt) -> [u8; 32] {
    let gt_bytes = format!("{gt:?}");
    let hk = Hkdf::<Sha256>::new(None, gt_bytes.as_bytes());
    let mut key = [0u8; 32];
    hk.expand(b"ibe-sym-key", &mut key)
        .expect("32 bytes is a valid HKDF output length");
    key
}

fn xor_bytes(key: &[u8; 32], data: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % 32])
        .collect()
}

// =============================================================================
// Encryptor — has access only to public information
// =============================================================================

pub struct Encryptor<'a> {
    params: &'a PublicParams,
}

impl<'a> Encryptor<'a> {
    pub fn new(params: &'a PublicParams) -> Self {
        Self { params }
    }

    /// Encrypt `plaintext` so that only the holder of `sig` can decrypt it.
    ///
    /// Algorithm (Boneh-Franklin IBE):
    ///   Q = H(pk_mpc || app_id)          -- G1, computable from public info
    ///   r <-- Zq*                         -- random ephemeral scalar
    ///   gt = e(r·Q, pk_mpc)              -- GT shared secret
    ///   U = r·G2                          -- sent to receiver
    ///   k = HKDF-SHA256(gt)              -- symmetric key
    ///   ciphertext = k XOR plaintext
    pub fn encrypt(&self, plaintext: &[u8]) -> Ciphertext {
        let q = hash_app_id_with_pk(&self.params.pk_mpc, &self.params.app_id);
        let r = Scalar::random(&mut OsRng);

        // Shared secret: gt = e(r·Q, pk_mpc)
        let gt =
            blstrs::pairing(&G1Affine::from(q * r), &G2Affine::from(&self.params.pk_mpc));

        // Ephemeral public value
        let u = G2Projective::generator() * r;

        let key = symmetric_key_from_gt(&gt);
        let bytes = xor_bytes(&key, plaintext);

        Ciphertext { u, bytes }
    }
}

// =============================================================================
// Decryptor — has access to the app private key (sig = msk · Q)
// =============================================================================

pub struct Decryptor {
    /// The app private key: sig = msk · H(pk_mpc || app_id)
    sig: G1Projective,
}

impl Decryptor {
    pub fn new(sig: G1Projective) -> Self {
        Self { sig }
    }

    /// Decrypt a ciphertext produced by `Encryptor::encrypt`.
    ///
    /// Algorithm:
    ///   gt = e(sig, U)
    ///      = e(msk·Q, r·G2)
    ///      = e(Q, G2)^(msk·r)
    ///      = e(r·Q, pk_mpc)       -- same shared secret as encryption
    ///   k = HKDF-SHA256(gt)
    ///   plaintext = k XOR ciphertext
    pub fn decrypt(&self, ct: &Ciphertext) -> Vec<u8> {
        let gt = blstrs::pairing(&G1Affine::from(&self.sig), &G2Affine::from(&ct.u));
        let key = symmetric_key_from_gt(&gt);
        xor_bytes(&key, &ct.bytes)
    }
}

// =============================================================================
// Demo
// =============================================================================

fn main() {
    println!("=== CKD Identity-Based Encryption Demo ===\n");

    // --- Simulate MPC: generate master keypair ---
    let msk = Scalar::random(&mut OsRng);
    let pk_mpc = G2Projective::generator() * msk;
    println!(
        "MPC master public key (G2): {}",
        hex::encode(G2Affine::from(&pk_mpc).to_compressed())
    );

    // --- Derive app_id (public, anyone can compute this) ---
    let account_id = "alice.near";
    let derivation_path = "my-app/v1";
    let app_id = derive_app_id(account_id, derivation_path);
    println!("account_id:      {account_id}");
    println!("derivation_path: {derivation_path}");
    println!("app_id:          {}", hex::encode(&app_id));

    let params = PublicParams { pk_mpc, app_id };

    // --- Simulate CKD: compute sig = msk · H(pk_mpc || app_id) ---
    // In reality this comes from the MPC network, encrypted and decrypted by the app.
    let hash_point = hash_app_id_with_pk(&params.pk_mpc, &params.app_id);
    let sig = hash_point * msk;
    println!(
        "\nApp private key (sig, G1): {}",
        hex::encode(G1Affine::from(&sig).to_compressed())
    );

    // -------------------------------------------------------------------------
    // ENCRYPTOR side: knows only (pk_mpc, app_id) — no sig, no msk
    // -------------------------------------------------------------------------
    let plaintext = b"secret message for alice";
    println!(
        "\n[Encryptor] plaintext:  \"{}\"",
        String::from_utf8_lossy(plaintext)
    );

    let encryptor = Encryptor::new(&params);
    let ciphertext = encryptor.encrypt(plaintext);

    println!(
        "[Encryptor] U (G2):     {}",
        hex::encode(G2Affine::from(&ciphertext.u).to_compressed())
    );
    println!("[Encryptor] ciphertext: {}", hex::encode(&ciphertext.bytes));

    // -------------------------------------------------------------------------
    // DECRYPTOR side: has sig, decrypts
    // -------------------------------------------------------------------------
    let decryptor = Decryptor::new(sig);
    let decrypted = decryptor.decrypt(&ciphertext);

    println!(
        "\n[Decryptor] decrypted:  \"{}\"",
        String::from_utf8_lossy(&decrypted)
    );
    assert_eq!(decrypted, plaintext, "decryption should recover the original plaintext");
    println!("[Decryptor] Matches original plaintext ✓");

    // -------------------------------------------------------------------------
    // Show that a wrong sig cannot decrypt
    // -------------------------------------------------------------------------
    let wrong_sig = hash_point * Scalar::random(&mut OsRng);
    let wrong_decryptor = Decryptor::new(wrong_sig);
    let wrong_decrypted = wrong_decryptor.decrypt(&ciphertext);

    println!(
        "\n[Decryptor] wrong sig decrypted: \"{}\"",
        String::from_utf8_lossy(&wrong_decrypted)
    );
    assert_ne!(wrong_decrypted, plaintext, "wrong sig should not decrypt correctly");
    println!("[Decryptor] Does not match original plaintext ✓ (decryption fails without correct sig)");

    println!("\nAll assertions passed.");
}
