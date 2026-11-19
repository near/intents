use std::{
    fmt::{self, Display},
    fs::File,
    io,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, anyhow};
use derive_more::From;
use digest::{Digest, Output, Update, array::ArraySize, common::OutputSize};
use digest_io::IoWrapper;
use impl_tools::autoimpl;

/// `[u8; N]`
pub type OutputArray<D> = <OutputSize<D> as ArraySize>::ArrayType<u8>;

/// Hash source
#[autoimpl(Debug, Clone, PartialEq, Eq where OutputArray<D>: trait)]
#[derive(From)]
pub enum HashSource<D: Digest> {
    /// Inline hash
    #[from(Output<D>)]
    Inline(OutputArray<D>),

    /// Hash contents of a file
    #[from(PathBuf, &Path)]
    File(PathBuf),

    /// Hash contents of stdin
    Stdin,
}

impl<D: Digest> HashSource<D> {
    pub fn hash(self) -> anyhow::Result<OutputArray<D>>
    where
        D: Update,
    {
        if let Self::Inline(hash) = self {
            return Ok(hash);
        }

        let mut hasher = IoWrapper(D::new());

        // write contents of the file to the wrapped hasher
        match self {
            Self::File(path) => File::open(&path)
                .and_then(|mut file| io::copy(&mut file, &mut hasher))
                .with_context(|| path.display().to_string())?,
            Self::Stdin => io::copy(&mut io::stdin(), &mut hasher)?,
            Self::Inline(_) => unreachable!(),
        };

        Ok(hasher.0.finalize().into())
    }
}

impl<D: Digest> Default for HashSource<D> {
    #[inline]
    fn default() -> Self {
        Self::Inline(Output::<D>::default().into())
    }
}

impl<D> FromStr for HashSource<D>
where
    D: Digest,
    OutputArray<D>: TryFrom<Vec<u8>>,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(path) = s.strip_prefix('@') {
            if path.is_empty() {
                return Err(anyhow!("expected a path after '@'"));
            }

            if path == "-" {
                return Ok(Self::Stdin);
            }

            return Ok(Self::File(path.into()));
        }

        let bytes = if let Some(s) = s.strip_prefix("0x") {
            hex::decode(s).context("hex")?
        } else {
            bs58::decode(s).into_vec().context("base58")?
        };

        let hash: OutputArray<D> = bytes.try_into().map_err(|_| {
            anyhow!(
                "HASH must be exactly {} bytes encoded via base58 or hex (with `0x` prefix)",
                <D as Digest>::output_size()
            )
        })?;

        Ok(Self::Inline(hash))
    }
}

impl<D: Digest> Display for HashSource<D> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inline(hash) => write!(f, "0x{}", hex::encode(hash)),
            Self::File(path) => write!(f, "@{}", path.display()),
            Self::Stdin => write!(f, "@-"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use sha2::Sha256;

    use super::*;

    const EMPTY_SHA256_HEX: &str =
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const HELLO_SHA256_HEX: &str =
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    fn output(hex_str: &str) -> OutputArray<Sha256> {
        hex::decode(hex_str).unwrap().try_into().unwrap()
    }

    fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("defuse-cli-utils-{name}-{}", std::process::id()));
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn from_str_parses_hex_with_0x_prefix() {
        let source: HashSource<Sha256> = format!("0x{EMPTY_SHA256_HEX}").parse().unwrap();
        assert_eq!(source, HashSource::Inline(output(EMPTY_SHA256_HEX)));
    }

    #[test]
    fn from_str_parses_base58() {
        let encoded = bs58::encode(hex::decode(EMPTY_SHA256_HEX).unwrap()).into_string();
        let source: HashSource<Sha256> = encoded.parse().unwrap();
        assert_eq!(source, HashSource::Inline(output(EMPTY_SHA256_HEX)));
    }

    #[test]
    fn from_str_rejects_wrong_length() {
        let err = "0x00".parse::<HashSource<Sha256>>().unwrap_err();
        assert_eq!(
            err.to_string(),
            "HASH must be exactly 32 bytes encoded via base58 or hex (with `0x` prefix)"
        );
    }

    #[test]
    fn from_str_reports_invalid_hex() {
        let err = "0xzz".parse::<HashSource<Sha256>>().unwrap_err();
        assert!(format!("{err:#}").starts_with("hex: "), "{err:#}");
    }

    #[test]
    fn from_str_reports_invalid_base58() {
        // `0`, `O`, `I` and `l` are not in the base58 alphabet
        let err = "0OIl".parse::<HashSource<Sha256>>().unwrap_err();
        assert!(format!("{err:#}").starts_with("base58: "), "{err:#}");
    }

    #[test]
    fn from_str_at_sign_selects_stdin_or_file() {
        assert_eq!(
            "@-".parse::<HashSource<Sha256>>().unwrap(),
            HashSource::Stdin
        );
        assert_eq!(
            "@some/file.wasm".parse::<HashSource<Sha256>>().unwrap(),
            HashSource::File(PathBuf::from("some/file.wasm"))
        );
    }

    #[test]
    fn from_str_rejects_bare_at_sign() {
        let err = "@".parse::<HashSource<Sha256>>().unwrap_err();
        assert_eq!(err.to_string(), "expected a path after '@'");
    }

    #[test]
    fn display_round_trips_through_from_str() {
        for source in [
            HashSource::<Sha256>::Inline(output(HELLO_SHA256_HEX)),
            HashSource::File(PathBuf::from("contract.wasm")),
            HashSource::Stdin,
        ] {
            let text = source.to_string();
            assert_eq!(
                text.parse::<HashSource<Sha256>>().unwrap(),
                source,
                "{text}"
            );
        }
    }

    #[test]
    fn default_is_all_zero_inline_hash() {
        assert_eq!(
            HashSource::<Sha256>::default(),
            HashSource::Inline(output(&"00".repeat(32)))
        );
    }

    #[test]
    fn hash_of_inline_is_returned_unchanged() {
        let hash = HashSource::<Sha256>::Inline(output(HELLO_SHA256_HEX))
            .hash()
            .unwrap();
        assert_eq!(hash, output(HELLO_SHA256_HEX));
    }

    #[test]
    fn hash_of_file_is_sha256_of_its_contents() {
        let path = temp_file("hash-of-file", b"hello");
        let hash = HashSource::<Sha256>::File(path.clone()).hash();
        fs::remove_file(&path).unwrap();
        assert_eq!(hash.unwrap(), output(HELLO_SHA256_HEX));
    }

    #[test]
    fn hash_of_missing_file_names_the_path() {
        let path =
            std::env::temp_dir().join(format!("defuse-cli-utils-missing-{}", std::process::id()));
        let err = HashSource::<Sha256>::File(path.clone()).hash().unwrap_err();
        assert!(
            format!("{err:#}").contains(&path.display().to_string()),
            "{err:#}"
        );
    }
}
