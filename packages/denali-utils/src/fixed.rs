#[derive(Clone, Copy)]
pub struct Fixed(i32);

impl std::fmt::Display for Fixed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<f64>::into(*self))
    }
}

impl PartialEq for Fixed {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Fixed {}

impl PartialOrd for Fixed {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Fixed {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl std::ops::Add for Fixed {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Fixed(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for Fixed {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for Fixed {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Fixed(self.0 - rhs.0)
    }
}

impl std::ops::SubAssign for Fixed {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::Mul for Fixed {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Fixed(((self.0 as i64 * rhs.0 as i64) >> 8) as i32)
    }
}

impl std::ops::MulAssign for Fixed {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl std::ops::Div for Fixed {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Fixed((((self.0 as i64) << 8) / rhs.0 as i64) as i32)
    }
}

impl std::ops::DivAssign for Fixed {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl std::ops::Neg for Fixed {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fixed(-self.0)
    }
}

impl From<f16> for Fixed {
    fn from(value: f16) -> Self {
        Fixed((value * 256.0).round() as i32)
    }
}

impl From<Fixed> for f16 {
    fn from(value: Fixed) -> Self {
        value.0 as f16 / 256.0
    }
}

impl From<f32> for Fixed {
    fn from(value: f32) -> Self {
        Fixed((value * 256.0).round() as i32)
    }
}

impl From<Fixed> for f32 {
    fn from(value: Fixed) -> Self {
        value.0 as f32 / 256.0
    }
}

impl From<f64> for Fixed {
    fn from(value: f64) -> Self {
        Fixed((value * 256.0).round() as i32)
    }
}

impl From<Fixed> for f64 {
    fn from(value: Fixed) -> Self {
        value.0 as f64 / 256.0
    }
}

impl From<f128> for Fixed {
    fn from(value: f128) -> Self {
        Fixed((value * 256.0).round() as i32)
    }
}

impl From<Fixed> for f128 {
    fn from(value: Fixed) -> Self {
        value.0 as f128 / 256.0
    }
}

impl From<i8> for Fixed {
    fn from(value: i8) -> Self {
        Fixed((value as i32) << 8)
    }
}

impl From<Fixed> for i8 {
    fn from(value: Fixed) -> Self {
        (value.0 / 256) as i8
    }
}

impl From<i16> for Fixed {
    fn from(value: i16) -> Self {
        Fixed((value as i32) << 8)
    }
}

impl From<Fixed> for i16 {
    fn from(value: Fixed) -> Self {
        (value.0 / 256) as i16
    }
}

impl From<i32> for Fixed {
    fn from(value: i32) -> Self {
        Fixed(value << 8)
    }
}

impl From<Fixed> for i32 {
    fn from(value: Fixed) -> Self {
        value.0 / 256
    }
}

impl From<i64> for Fixed {
    fn from(value: i64) -> Self {
        Fixed((value as i32) << 8)
    }
}

impl From<Fixed> for i64 {
    fn from(value: Fixed) -> Self {
        (value.0 / 256) as i64
    }
}

impl From<i128> for Fixed {
    fn from(value: i128) -> Self {
        Fixed((value as i32) << 8)
    }
}

impl From<Fixed> for i128 {
    fn from(value: Fixed) -> Self {
        (value.0 / 256) as i128
    }
}

impl From<isize> for Fixed {
    fn from(value: isize) -> Self {
        Fixed((value as i32) << 8)
    }
}

impl From<Fixed> for isize {
    fn from(value: Fixed) -> Self {
        (value.0 / 256) as isize
    }
}

impl From<u8> for Fixed {
    fn from(value: u8) -> Self {
        Fixed((value as i32) << 8)
    }
}

impl From<Fixed> for u8 {
    fn from(value: Fixed) -> Self {
        (value.0 >> 8) as u8
    }
}

impl From<u16> for Fixed {
    fn from(value: u16) -> Self {
        Fixed((value as i32) << 8)
    }
}

impl From<Fixed> for u16 {
    fn from(value: Fixed) -> Self {
        (value.0 >> 8) as u16
    }
}

impl From<u32> for Fixed {
    fn from(value: u32) -> Self {
        Fixed((value as i32) << 8)
    }
}

impl From<Fixed> for u32 {
    fn from(value: Fixed) -> Self {
        (value.0 >> 8) as u32
    }
}

impl From<u64> for Fixed {
    fn from(value: u64) -> Self {
        Fixed((value as i32) << 8)
    }
}

impl From<Fixed> for u64 {
    fn from(value: Fixed) -> Self {
        (value.0 >> 8) as u64
    }
}

impl From<u128> for Fixed {
    fn from(value: u128) -> Self {
        Fixed((value as i32) << 8)
    }
}

impl From<Fixed> for u128 {
    fn from(value: Fixed) -> Self {
        (value.0 >> 8) as u128
    }
}

impl From<usize> for Fixed {
    fn from(value: usize) -> Self {
        Fixed((value as i32) << 8)
    }
}

impl From<Fixed> for usize {
    fn from(value: Fixed) -> Self {
        (value.0 >> 8) as usize
    }
}

impl Fixed {
    pub const fn abs(self) -> Fixed {
        Fixed(self.0.abs())
    }
}

#[cfg(test)]
mod tests {
    use crate::fixed::Fixed;

    #[test]
    fn ints() {
        let fix = Fixed::from(54.34);

        assert_eq!(54_i8, fix.into());
        assert_eq!(54_i16, fix.into());
        assert_eq!(54_i32, fix.into());
        assert_eq!(54_i64, fix.into());
        assert_eq!(54_i128, fix.into());
        assert_eq!(54_isize, fix.into());

        assert_eq!(54_u8, fix.into());
        assert_eq!(54_u16, fix.into());
        assert_eq!(54_u32, fix.into());
        assert_eq!(54_u64, fix.into());
        assert_eq!(54_u128, fix.into());
        assert_eq!(54_usize, fix.into());
    }

    #[test]
    fn negative_ints() {
        let fix = Fixed::from(-23);

        assert_eq!(-23_i8, fix.into());
        assert_eq!(-23_i16, fix.into());
        assert_eq!(-23_i32, fix.into());
        assert_eq!(-23_i64, fix.into());
        assert_eq!(-23_i128, fix.into());
        assert_eq!(-23_isize, fix.into());
    }

    #[test]
    fn floats() {
        let fix = Fixed::from(20.456);

        assert!((Into::<f16>::into(fix) - 20.456_f16).abs() < 0.01);
        assert!((Into::<f32>::into(fix) - 20.456_f32).abs() < 0.01);
        assert!((Into::<f64>::into(fix) - 20.456_f64).abs() < 0.01);
        assert!((Into::<f128>::into(fix) - 20.456_f128).abs() < 0.01);
    }

    #[test]
    fn negative_floats() {
        let fix = Fixed::from(-10.2);

        assert!((Into::<f16>::into(fix) + 10.2_f16).abs() < 0.01);
        assert!((Into::<f32>::into(fix) + 10.2_f32).abs() < 0.01);
        assert!((Into::<f64>::into(fix) + 10.2_f64).abs() < 0.01);
        assert!((Into::<f128>::into(fix) + 10.2_f128).abs() < 0.01);
    }

    #[test]
    fn add_sub() {
        let mut fix1 = Fixed::from(12.5);
        let fix2 = Fixed::from(7.5);

        assert_eq!(20, (fix1 + fix2).into()); // add
        assert_eq!(5, (fix1 - fix2).into()); // sub

        fix1 += fix2;
        assert_eq!(20, fix1.into()); // add assign
        fix1 -= fix2;
        assert_eq!(12.5, fix1.into()); // sub assign
    }

    #[test]
    fn mul_div() {
        let mut fix1 = Fixed::from(10);
        let fix2 = Fixed::from(2);

        assert_eq!(20, (fix1 * fix2).into()); // add
        assert_eq!(5, (fix1 / fix2).into()); // sub

        fix1 *= fix2;
        assert_eq!(20, fix1.into()); // add assign
        fix1 /= fix2;
        assert_eq!(10, fix1.into()); // sub assign
    }

    #[test]
    fn neg_abs() {
        let fix = Fixed::from(12.5);
        assert_eq!(12.5, fix.into());
        assert_eq!(-12.5, (-fix).into());
        assert_eq!(12.5, (-fix).abs().into());
    }
}
