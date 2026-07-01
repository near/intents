use std::{
    fmt::{self, Display},
    fs::File,
    io,
    path::PathBuf,
    str::FromStr,
};

use anyhow::{Context, anyhow};
use derive_more::From;
use digest::{Digest, Output, Update, array::ArraySize, common::OutputSize};
use digest_io::IoWrapper;
use impl_tools::autoimpl;

/// `[u8; N]`
pub type OutputArray<D> = <OutputSize<D> as ArraySize>::ArrayType<u8>;

#[autoimpl(Debug, Clone, PartialEq, Eq where OutputArray<D>: trait)]
#[derive(From)]
/// Hash source
pub enum HashSource<D: Digest> {
    /// Inline hash
    #[from(Output<D>)]
    Inline(OutputArray<D>),

    /// Hash contents of a file
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

impl<D: Digest> FromStr for HashSource<D>
where
    OutputArray<D>: TryFrom<Vec<u8>>,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "-" {
            return Ok(Self::Stdin);
        }

        if let Some(path) = s.strip_prefix('@') {
            if path.is_empty() {
                return Err(anyhow!("expected a path after '@'"));
            }
            return Ok(Self::File(path.into()));
        }

        let bytes = if let Some(s) = s.strip_prefix("0x") {
            hex::decode(s).context("hex")?
        } else {
            bs58::decode(s).into_vec().context("base58")?
        };

        let hash: OutputArray<D> = bytes
            .try_into()
            .map_err(|_| anyhow!("HASH must be 32 bytes encoded via hex or base58"))?;

        Ok(Self::Inline(hash))
    }
}

impl<D: Digest> Display for HashSource<D> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inline(hash) => write!(f, "0x{}", hex::encode(hash)),
            Self::File(path) => write!(f, "@{}", path.display()),
            Self::Stdin => write!(f, "-"),
        }
    }
}
