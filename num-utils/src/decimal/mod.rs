mod ops;
mod str;

#[derive(Debug, Clone, Copy, Eq, Hash)]
pub struct D120([u8; Self::SIZE]);

impl D120 {
    const SIZE: usize = size_of::<u128>();
    const BASE: u128 = 10;
    const MAX_DIGITS: u128 = (1 << 120) - 1;
    const MAX_DECIMALS: u8 = Self::MAX_DIGITS.ilog(Self::BASE) as u8;

    pub const MIN: Self = Self::new(true, 0, Self::MAX_DIGITS).unwrap();
    pub const MAX: Self = Self::new(false, 0, Self::MAX_DIGITS).unwrap();

    pub const ZERO: Self = Self::new(false, 0, 0).unwrap();
    pub const ONE: Self = Self::new(false, 0, 1).unwrap();

    #[inline]
    pub const fn new(is_negative: bool, decimals: u8, digits: u128) -> Option<Self> {
        if decimals > Self::MAX_DECIMALS || digits > Self::MAX_DIGITS {
            return None;
        }

        Some(unsafe { Self::new_unchecked(is_negative, decimals, digits) })
    }

    #[inline]
    const unsafe fn new_unchecked(is_negative: bool, decimals: u8, digits: u128) -> Self {
        let mut bytes = digits.to_be_bytes();
        bytes[0] = (is_negative as u8) << 7 | decimals;
        Self(bytes)
    }

    #[inline]
    pub const fn from_i128(n: i128) -> Option<Self> {
        Self::new(n.is_negative(), 0, n.unsigned_abs())
    }

    #[inline]
    pub const fn from_u128(n: u128) -> Option<Self> {
        Self::new(false, 0, n)
    }

    #[inline]
    pub const fn is_sign_negative(self) -> bool {
        self.0[0] & (1 << 7) == 1
    }

    #[inline]
    pub const fn is_sign_positive(self) -> bool {
        !self.is_sign_negative()
    }

    #[inline]
    pub const fn digits(self) -> u128 {
        u128::from_be_bytes(self.0) & Self::MAX_DIGITS
    }

    #[inline]
    pub const fn decimals(self) -> u8 {
        self.0[0] & ((1 << 7) - 1)
    }

    const fn denominator(self) -> u128 {
        Self::BASE.pow(self.decimals() as u32)
    }

    #[inline]
    pub const fn is_zero(self) -> bool {
        self.digits() == 0
    }

    #[inline]
    pub const fn trunc(self) -> i128 {
        -1i128.pow(self.is_sign_negative() as u32) * self.digits() as i128
            / self.denominator() as i128
    }

    #[inline]
    pub const fn rescale(self, decimals: u8) -> Option<Self> {
        if decimals > Self::MAX_DECIMALS {
            return None;
        }
        Some(unsafe { self.rescale_unchecked(decimals) })
    }

    #[inline]
    const unsafe fn rescale_unchecked(mut self, decimals: u8) -> Self {
        self.0[0] = self.0[0] & (1 << 7) | decimals;
        self
    }

    #[inline]
    pub const fn reduce(self) -> Self {
        let mut digits = self.digits();
        if digits == 0 {
            return Self::ZERO;
        }

        let mut decimals = self.decimals();
        while decimals > 0 && digits % Self::BASE == 0 {
            digits /= Self::BASE;
            decimals -= 1;
        }
        unsafe { Self::new_unchecked(self.is_sign_negative(), decimals, digits) }
    }

    #[inline]
    pub const fn neg(mut self) -> Self {
        self.0[0] ^= 1 << 7;
        self
    }

    #[inline]
    pub const fn ceil(self) -> u128 {
        todo!()
    }

    #[inline]
    pub const fn round(self) -> u128 {
        todo!()
    }

    #[inline]
    pub const fn to_f32(self) -> f32 {
        self.digits() as f32 / Self::BASE.pow(self.decimals() as u32) as f32
    }

    #[inline]
    pub const fn to_f64(self) -> f64 {
        self.digits() as f64 / Self::BASE.pow(self.decimals() as u32) as f64
    }

    #[inline]
    pub const fn to_be_bytes(self) -> [u8; Self::SIZE] {
        self.0
    }

    #[inline]
    pub const fn from_be_bytes(bytes: [u8; Self::SIZE]) -> Self {
        Self(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct UD120(D120);
