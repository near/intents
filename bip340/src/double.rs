use digest::{FixedOutput, HashMarker, OutputSizeUser, Update};

#[derive(Debug, Clone, Default)]
pub struct Double<D>(D);

impl<D> Update for Double<D>
where
    D: Update,
{
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
}

impl<D> OutputSizeUser for Double<D>
where
    D: OutputSizeUser,
{
    type OutputSize = D::OutputSize;
}

impl<D> FixedOutput for Double<D>
where
    D: FixedOutput + Update + Default,
{
    fn finalize_into(self, out: &mut digest::Output<Self>) {
        D::default()
            .chain(self.0.finalize_fixed())
            .finalize_into(out);
    }
}

impl<D> HashMarker for Double<D> where D: HashMarker {}

// TODO: tests

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;
    use sha2::{Digest, Sha256};

    use super::*;

    #[rstest]
    #[case(b"", hex!("5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456"))]
    fn double_sha256(#[case] input: &[u8], #[case] output: [u8; 32]) {
        assert_eq!(Double::<Sha256>::digest(input), output.into());
    }
}
