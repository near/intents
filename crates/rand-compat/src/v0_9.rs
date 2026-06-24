pub use rand_core_0_9;

#[derive(Debug)]
pub struct V0_9<R>(pub R);

impl<R: rand_core_0_9::TryRngCore> rand_core_0_9::TryRngCore for V0_9<R> {
    type Error = R::Error;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        self.0.try_next_u32()
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        self.0.try_next_u64()
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        self.0.try_fill_bytes(dst)
    }
}

impl<R: rand_core_0_9::TryCryptoRng> rand_core_0_9::TryCryptoRng for V0_9<R> {}

impl<R: rand_core_0_9::SeedableRng> rand_core_0_9::SeedableRng for V0_9<R> {
    type Seed = R::Seed;

    fn from_seed(seed: Self::Seed) -> Self {
        Self(R::from_seed(seed))
    }
}

#[cfg(feature = "rand_core_0_6")]
const _: () = {
    impl<R: rand_core_0_9::RngCore> crate::rand_core_0_6::RngCore for V0_9<R> {
        fn next_u32(&mut self) -> u32 {
            self.0.next_u32()
        }

        fn next_u64(&mut self) -> u64 {
            self.0.next_u64()
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            self.0.fill_bytes(dest);
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), crate::rand_core_0_6::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    impl<R: rand_core_0_9::TryCryptoRng> crate::rand_core_0_6::CryptoRng for V0_9<R> {}

    impl<R: rand_core_0_9::SeedableRng> crate::rand_core_0_6::SeedableRng for V0_9<R> {
        type Seed = R::Seed;

        fn from_seed(seed: Self::Seed) -> Self {
            Self(R::from_seed(seed))
        }
    }
};

#[cfg(feature = "rand_core_0_10")]
const _: () = {
    impl<R> crate::rand_core_0_10::TryRng for V0_9<R>
    where
        R: rand_core_0_9::TryRngCore,
        R::Error: core::error::Error,
    {
        type Error = R::Error;

        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            self.0.try_next_u32()
        }

        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            self.0.try_next_u64()
        }

        fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
            self.0.try_fill_bytes(dst)
        }
    }

    impl<R> crate::rand_core_0_10::TryCryptoRng for V0_9<R>
    where
        R: rand_core_0_9::TryCryptoRng,
        Self: crate::rand_core_0_10::TryRng,
    {
    }

    impl<R: rand_core_0_9::SeedableRng> crate::rand_core_0_10::SeedableRng for V0_9<R> {
        type Seed = R::Seed;

        fn from_seed(seed: Self::Seed) -> Self {
            Self(R::from_seed(seed))
        }
    }
};
