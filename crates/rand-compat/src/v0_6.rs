pub use rand_core_0_6;

#[derive(Debug)]
pub struct V0_6<R>(pub R);

impl<R: rand_core_0_6::RngCore> rand_core_0_6::RngCore for V0_6<R> {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core_0_6::Error> {
        self.0.try_fill_bytes(dest)
    }
}

impl<R: rand_core_0_6::CryptoRng> rand_core_0_6::CryptoRng for V0_6<R> {}

impl<R: rand_core_0_6::SeedableRng> rand_core_0_6::SeedableRng for V0_6<R> {
    type Seed = R::Seed;

    fn from_seed(seed: Self::Seed) -> Self {
        Self(R::from_seed(seed))
    }
}

#[cfg(feature = "rand_core_0_9")]
const _: () = {
    use core::convert::Infallible;

    impl<R: rand_core_0_6::RngCore> crate::rand_core_0_9::TryRngCore for V0_6<R> {
        type Error = Infallible;

        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            Ok(self.0.next_u32())
        }

        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            Ok(self.0.next_u64())
        }

        fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
            self.0.fill_bytes(dst);
            Ok(())
        }
    }

    impl<R: rand_core_0_6::RngCore + rand_core_0_6::CryptoRng> crate::rand_core_0_9::TryCryptoRng
        for V0_6<R>
    {
    }

    impl<R> crate::rand_core_0_9::SeedableRng for V0_6<R>
    where
        R: rand_core_0_6::SeedableRng,
        R::Seed: Clone + AsRef<[u8]>,
    {
        type Seed = R::Seed;

        fn from_seed(seed: Self::Seed) -> Self {
            Self(R::from_seed(seed))
        }
    }
};

#[cfg(feature = "rand_core_0_10")]
const _: () = {
    use core::convert::Infallible;

    impl<R: rand_core_0_6::RngCore> crate::rand_core_0_10::TryRng for V0_6<R> {
        type Error = Infallible;

        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            Ok(self.0.next_u32())
        }

        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            Ok(self.0.next_u64())
        }

        fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
            self.0.fill_bytes(dst);
            Ok(())
        }
    }

    impl<R: rand_core_0_6::RngCore + rand_core_0_6::CryptoRng> crate::rand_core_0_10::TryCryptoRng
        for V0_6<R>
    {
    }

    impl<R> crate::rand_core_0_10::SeedableRng for V0_6<R>
    where
        R: rand_core_0_6::SeedableRng,
        R::Seed: Clone + AsRef<[u8]>,
    {
        type Seed = R::Seed;

        fn from_seed(seed: Self::Seed) -> Self {
            Self(R::from_seed(seed))
        }
    }
};
