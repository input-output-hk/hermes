//! Signed Integer Multiply and

use catalyst_types::conversion::from_saturating;

/// A trait providing a saturating multiply-accumulate:
///
/// `self + (y * z)` with saturating arithmetic, updating `Self`.
///
/// `self` is the Accumulator, and can be any integer but will saturate at max i128 if
/// its a `u128` or larger.  Otherwise it will saturate within the bounds of its type.
///
/// `y` and `z` can be any integer type.
///
/// Works across any combination of integer types (signed or unsigned).
pub trait SaturatingMulAdd<U, V> {
    /// Multiply and Accumulate Integers.
    fn mul_add(
        &mut self,
        y: U,
        z: V,
    );
}

/// Implement the trait for various integer types.
macro_rules! impl_saturating_mul_add {
    ($($t:ty),*) => {
        $(
            impl<U, V> SaturatingMulAdd<U, V> for $t
            where
                U: Copy
                    + TryInto<i128>
                    + std::ops::Sub<Output = U>
                    + std::cmp::PartialOrd<U>
                    + num_traits::identities::Zero,
                V: Copy
                    + TryInto<i128>
                    + std::ops::Sub<Output = V>
                    + std::cmp::PartialOrd<V>
                    + num_traits::identities::Zero,
            {
                fn mul_add(&mut self, y: U, z: V) {
                    let self128: i128 = from_saturating(*self);
                    let y128:i128 = from_saturating(y);
                    let z128:i128 = from_saturating(z);

                    *self = from_saturating(z128.saturating_mul(y128).saturating_add(self128));
                }
            }
        )*
    };
}

// Implement for all signed integer types
impl_saturating_mul_add!(i8, i16, i32, i64, i128, isize);
// Implement for all unsigned integer types
impl_saturating_mul_add!(u8, u16, u32, u64, usize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i32_u64_i8() {
        let mut s: i32 = 5;
        let y: u128 = 10;
        let z: i8 = -3;
        s.mul_add(y, z);
        assert_eq!(s, -25);
    }

    #[test]
    fn u8_i64_u128_pos() {
        let mut s: u8 = 5;
        let y: i64 = 10000;
        let z: u128 = 1_234_567;
        s.mul_add(y, z);
        assert_eq!(s, 255);
    }

    #[test]
    fn u8_i64_u128_neg() {
        let mut s: u8 = 5;
        let y: i64 = -10000;
        let z: u128 = 1_234_567;
        s.mul_add(y, z);
        assert_eq!(s, 0);
    }

    #[test]
    fn i16_i64_u128_neg() {
        let mut s: i16 = 5;
        let y: i64 = -10000;
        let z: u128 = 1_234_567;
        s.mul_add(y, z);
        assert_eq!(s, -32768);
    }
}
