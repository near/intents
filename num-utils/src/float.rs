use tlbits::bitvec::{array::BitArray, order::Msb0};

pub struct F128(BitArray<[u8; 16], Msb0>);

impl F128 {
    fn is_negative(&self) -> bool {
        *unsafe { self.0.get_unchecked(0) }
    }

    fn exponent(&self) -> u8 {
        *unsafe { self.0.as_raw_slice().get_unchecked(0) } & 0b0111111
    }
    
    // fn to_f64(&self) {
    //     f64::from_be_bytes(bytes)
    // }
}


