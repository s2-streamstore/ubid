#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::{array::TryFromSliceError, fmt, ops::Deref, str::FromStr};

use fast32::base32;

/// A marker trait for standard UBID byte widths.
///
/// This trait is implemented for `()` at the standard byte widths: 5, 10, 15, and 20 bytes.
/// Generic code can use a `where (): StandardWidth<N>` bound to accept any supported UBID width.
///
/// ```
/// use ubid::{StandardWidth, Ubid};
///
/// fn as_text<const N: usize>(id: Ubid<N>) -> String
/// where
///     (): StandardWidth<N>,
/// {
///     id.to_string()
/// }
/// ```
pub trait StandardWidth<const N: usize> {}

impl StandardWidth<5> for () {}
impl StandardWidth<10> for () {}
impl StandardWidth<15> for () {}
impl StandardWidth<20> for () {}

/// A 40-bit UBID backed by 5 random bytes.
pub type Ubid40 = Ubid<5>;

/// An 80-bit UBID backed by 10 random bytes.
pub type Ubid80 = Ubid<10>;

/// A 120-bit UBID backed by 15 random bytes.
pub type Ubid120 = Ubid<15>;

/// A 160-bit UBID backed by 20 random bytes.
pub type Ubid160 = Ubid<20>;

/// An error returned when decoding a UBID from its text representation fails.
#[derive(Debug)]
pub struct DecodeError(fast32::DecodeError);

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for DecodeError {}

/// A fixed-width random identifier backed by exactly `N` bytes.
///
/// The const parameter `N` is a byte count and must be one of the standard UBID widths: 5, 10, 15,
/// or 20 bytes. For example, `Ubid<15>` is a 120-bit identifier and is available as the `Ubid120`
/// alias.
#[derive(PartialEq, PartialOrd, Eq, Ord, Hash, Clone, Copy)]
pub struct Ubid<const N: usize>([u8; N])
where
    (): StandardWidth<N>;

impl<const N: usize> Ubid<N>
where
    (): StandardWidth<N>,
{
    /// Generates a new UBID using the operating system's random number source.
    ///
    /// # Panics
    ///
    /// Panics if the operating system fails to provide random bytes.
    pub fn generate() -> Ubid<N> {
        Self::try_generate().expect("failed to generate UBID using OS randomness")
    }

    /// Attempts to generate a new UBID using the operating system's random number source.
    pub fn try_generate() -> Result<Ubid<N>, getrandom::Error> {
        let mut bytes = [0; N];
        getrandom::fill(&mut bytes)?;
        Ok(Ubid(bytes))
    }

    /// Encodes this UBID as lowercase Crockford base32.
    pub fn encode(&self) -> String {
        base32::CROCKFORD_LOWER.encode(&self.0)
    }

    /// Decodes a UBID from Crockford base32.
    ///
    /// Encoded UBIDs are canonical lowercase. Decoding is tolerant and accepts Crockford aliases,
    /// including uppercase letters, `o`/`O` for `0`, and `i`/`l`/`I`/`L` for `1`.
    ///
    /// Decoding fails if the input is not valid base32 or does not decode to exactly `N` bytes.
    pub fn decode(s: &str) -> Result<Ubid<N>, DecodeError> {
        let bytes = base32::CROCKFORD_LOWER.decode_str(s).map_err(DecodeError)?;
        let ubid = bytes
            .try_into()
            .map(Self)
            .map_err(|b: Vec<u8>| fast32::DecodeError::InvalidLength { length: b.len() })
            .map_err(DecodeError)?;
        Ok(ubid)
    }

    /// Returns the size of this UBID in bytes.
    pub const fn size() -> usize {
        N
    }
}

impl<const N: usize> TryFrom<&str> for Ubid<N>
where
    (): StandardWidth<N>,
{
    type Error = DecodeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ubid::decode(value)
    }
}

impl<const N: usize> FromStr for Ubid<N>
where
    (): StandardWidth<N>,
{
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ubid::decode(s)
    }
}

impl<const N: usize> fmt::Display for Ubid<N>
where
    (): StandardWidth<N>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

impl<const N: usize> fmt::Debug for Ubid<N>
where
    (): StandardWidth<N>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ubid{}{{{}}}", N * 8, &self.encode())
    }
}

impl<const N: usize> Deref for Ubid<N>
where
    (): StandardWidth<N>,
{
    type Target = [u8; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> AsRef<[u8; N]> for Ubid<N>
where
    (): StandardWidth<N>,
{
    fn as_ref(&self) -> &[u8; N] {
        &self.0
    }
}

impl<const N: usize> From<[u8; N]> for Ubid<N>
where
    (): StandardWidth<N>,
{
    fn from(value: [u8; N]) -> Self {
        Ubid(value)
    }
}

impl<const N: usize> TryFrom<&[u8]> for Ubid<N>
where
    (): StandardWidth<N>,
{
    type Error = TryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        value.try_into().map(Ubid)
    }
}

#[cfg(any(feature = "bytes", test))]
impl<const N: usize> From<Ubid<N>> for bytes::Bytes
where
    (): StandardWidth<N>,
{
    fn from(ubid: Ubid<N>) -> Self {
        bytes::Bytes::copy_from_slice(&ubid.0)
    }
}

#[cfg(feature = "proptest")]
impl<const N: usize> proptest::arbitrary::Arbitrary for Ubid<N>
where
    (): StandardWidth<N>,
{
    type Parameters = ();
    type Strategy =
        proptest::strategy::MapInto<<[u8; N] as proptest::arbitrary::Arbitrary>::Strategy, Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        any::<[u8; N]>().prop_map_into()
    }
}

#[cfg(test)]
mod test {
    use proptest::prelude::*;

    use super::*;

    proptest!(
        #[test]
        fn roundtrip_str(input in prop::array::uniform10(0u8..)) {
            let ubid: Ubid80 = Ubid(input);
            let encoded = ubid.encode();
            prop_assert_eq!(16, encoded.len());
            let decoded: Ubid<10> = Ubid::decode(&encoded).unwrap();
            prop_assert_eq!(ubid, decoded);
        }
    );

    proptest!(
        #[test]
        fn roundtrip_bytes(input in prop::array::uniform20(0u8..)) {
            let ubid = Ubid160::from(input);
            let bytes: bytes::Bytes = ubid.into();
            let converted: Ubid160 = bytes.as_ref().try_into().unwrap();
            prop_assert_eq!(ubid, converted);
        }
    );

    #[cfg(feature = "proptest")]
    proptest!(
        #[test]
        fn arbitrary_roundtrip(ubid in any::<Ubid160>()) {
            let decoded: Ubid160 = ubid.to_string().parse().unwrap();
            prop_assert_eq!(ubid, decoded);
        }
    );
}
