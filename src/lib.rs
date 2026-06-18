#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::{array::TryFromSliceError, fmt, ops::Deref, str::FromStr};

use rand::Rng;

const CROCKFORD_LOWER: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
const CROCKFORD_LOWER_LETTERS: &[u8; 22] = b"abcdefghjkmnpqrstvwxyz";
const INVALID_SYMBOL: u8 = u8::MAX;
const MAX_ENCODED_LEN: usize = 32;
const CROCKFORD_DECODE: [u8; 256] = crockford_decode_table();

const fn crockford_decode_table() -> [u8; 256] {
    let mut table = [INVALID_SYMBOL; 256];

    let mut digit = 0;
    while digit < 10 {
        table[b'0' as usize + digit] = digit as u8;
        digit += 1;
    }

    let mut index = 0;
    while index < CROCKFORD_LOWER_LETTERS.len() {
        let lower = CROCKFORD_LOWER_LETTERS[index];
        let value = 10 + index as u8;
        table[lower as usize] = value;
        table[(lower - b'a' + b'A') as usize] = value;
        index += 1;
    }

    table[b'i' as usize] = 1;
    table[b'I' as usize] = 1;
    table[b'l' as usize] = 1;
    table[b'L' as usize] = 1;
    table[b'o' as usize] = 0;
    table[b'O' as usize] = 0;

    table
}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(DecodeErrorKind);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecodeErrorKind {
    InvalidChar { byte: u8, index: usize },
    InvalidLength { length: usize },
}

impl DecodeError {
    fn invalid_char(byte: u8, index: usize) -> Self {
        DecodeError(DecodeErrorKind::InvalidChar { byte, index })
    }

    fn invalid_length(length: usize) -> Self {
        DecodeError(DecodeErrorKind::InvalidLength { length })
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            DecodeErrorKind::InvalidChar { byte, index } => {
                write!(f, "Invalid char of '{}' at position {index}", byte as char)
            }
            DecodeErrorKind::InvalidLength { length } => {
                write!(f, "Invalid length of {length}")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

fn encode_into_buffer<'a, const N: usize>(
    bytes: &[u8; N],
    out: &'a mut [u8; MAX_ENCODED_LEN],
) -> &'a str
where
    (): StandardWidth<N>,
{
    let encoded_len = N / 5 * 8;

    for (chunk, encoded_chunk) in bytes
        .chunks_exact(5)
        .zip(out[..encoded_len].chunks_exact_mut(8))
    {
        let [b0, b1, b2, b3, b4] = chunk.try_into().expect("chunks are exactly 5 bytes");

        encoded_chunk[0] = CROCKFORD_LOWER[(b0 >> 3) as usize];
        encoded_chunk[1] = CROCKFORD_LOWER[(((b0 & 0b0000_0111) << 2) | (b1 >> 6)) as usize];
        encoded_chunk[2] = CROCKFORD_LOWER[((b1 & 0b0011_1110) >> 1) as usize];
        encoded_chunk[3] = CROCKFORD_LOWER[(((b1 & 0b0000_0001) << 4) | (b2 >> 4)) as usize];
        encoded_chunk[4] = CROCKFORD_LOWER[(((b2 & 0b0000_1111) << 1) | (b3 >> 7)) as usize];
        encoded_chunk[5] = CROCKFORD_LOWER[((b3 & 0b0111_1100) >> 2) as usize];
        encoded_chunk[6] = CROCKFORD_LOWER[(((b3 & 0b0000_0011) << 3) | (b4 >> 5)) as usize];
        encoded_chunk[7] = CROCKFORD_LOWER[(b4 & 0b0001_1111) as usize];
    }

    // SAFETY: every byte written to `out` comes from the ASCII Crockford alphabet above.
    unsafe { std::str::from_utf8_unchecked(&out[..encoded_len]) }
}

fn decode_symbol(byte: u8, index: usize) -> Result<u8, DecodeError> {
    let value = CROCKFORD_DECODE[byte as usize];
    if value == INVALID_SYMBOL {
        Err(DecodeError::invalid_char(byte, index))
    } else {
        Ok(value)
    }
}

fn decode_to_array<const N: usize>(s: &str) -> Result<[u8; N], DecodeError>
where
    (): StandardWidth<N>,
{
    let encoded_len = N / 5 * 8;
    if s.len() != encoded_len {
        return Err(DecodeError::invalid_length(s.len()));
    }

    let mut decoded = [0; N];

    for (chunk_index, (encoded_chunk, decoded_chunk)) in s
        .as_bytes()
        .chunks_exact(8)
        .zip(decoded.chunks_exact_mut(5))
        .enumerate()
    {
        let offset = chunk_index * 8;
        let b0 = decode_symbol(encoded_chunk[0], offset)?;
        let b1 = decode_symbol(encoded_chunk[1], offset + 1)?;
        let b2 = decode_symbol(encoded_chunk[2], offset + 2)?;
        let b3 = decode_symbol(encoded_chunk[3], offset + 3)?;
        let b4 = decode_symbol(encoded_chunk[4], offset + 4)?;
        let b5 = decode_symbol(encoded_chunk[5], offset + 5)?;
        let b6 = decode_symbol(encoded_chunk[6], offset + 6)?;
        let b7 = decode_symbol(encoded_chunk[7], offset + 7)?;

        decoded_chunk[0] = (b0 << 3) | (b1 >> 2);
        decoded_chunk[1] = ((b1 & 0b0000_0011) << 6) | (b2 << 1) | (b3 >> 4);
        decoded_chunk[2] = ((b3 & 0b0000_1111) << 4) | (b4 >> 1);
        decoded_chunk[3] = ((b4 & 0b0000_0001) << 7) | (b5 << 2) | (b6 >> 3);
        decoded_chunk[4] = ((b6 & 0b0000_0111) << 5) | b7;
    }

    Ok(decoded)
}

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
    /// Generates a new UBID using the thread-local random number generator.
    pub fn generate() -> Ubid<N> {
        Self::generate_with(&mut rand::rng())
    }

    /// Generates a new UBID using the supplied random number generator.
    ///
    /// This is useful for deterministic tests, simulations, and callers that need to control their
    /// randomness source.
    pub fn generate_with(rng: &mut impl Rng) -> Ubid<N> {
        let mut bytes = [0; N];
        rng.fill_bytes(&mut bytes);
        Ubid(bytes)
    }

    /// Encodes this UBID as lowercase Crockford base32.
    pub fn encode(&self) -> String {
        let mut encoded = [0; MAX_ENCODED_LEN];
        encode_into_buffer(&self.0, &mut encoded).to_owned()
    }

    /// Decodes a UBID from Crockford base32.
    ///
    /// Encoded UBIDs are canonical lowercase. Decoding is tolerant and accepts Crockford aliases,
    /// including uppercase letters, `o`/`O` for `0`, and `i`/`l`/`I`/`L` for `1`.
    ///
    /// Decoding fails if the input is not valid base32 or does not decode to exactly `N` bytes.
    pub fn decode(s: &str) -> Result<Ubid<N>, DecodeError> {
        decode_to_array(s).map(Self)
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
        let mut encoded = [0; MAX_ENCODED_LEN];
        f.write_str(encode_into_buffer(&self.0, &mut encoded))
    }
}

impl<const N: usize> fmt::Debug for Ubid<N>
where
    (): StandardWidth<N>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut encoded = [0; MAX_ENCODED_LEN];
        write!(
            f,
            "Ubid{}{{{}}}",
            N * 8,
            encode_into_buffer(&self.0, &mut encoded)
        )
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

#[cfg(feature = "serde")]
impl<const N: usize> serde::Serialize for Ubid<N>
where
    (): StandardWidth<N>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut encoded = [0; MAX_ENCODED_LEN];
        serializer.serialize_str(encode_into_buffer(&self.0, &mut encoded))
    }
}

#[cfg(feature = "serde")]
impl<'de, const N: usize> serde::Deserialize<'de> for Ubid<N>
where
    (): StandardWidth<N>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct UbidVisitor<const N: usize>;

        impl<'de, const N: usize> serde::de::Visitor<'de> for UbidVisitor<N>
        where
            (): StandardWidth<N>,
        {
            type Value = Ubid<N>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "a {}-character Crockford base32 UBID string", N / 5 * 8)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ubid::decode(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(UbidVisitor::<N>)
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

    fn assert_decode_variants_match_fast32<const N: usize>(input: [u8; N])
    where
        (): StandardWidth<N>,
    {
        let encoded = fast32::base32::CROCKFORD_LOWER.encode(&input);
        let variants = [
            encoded.clone(),
            encoded.to_uppercase(),
            encoded.replace('0', "o").replace('1', "i"),
            encoded.replace('0', "O").replace('1', "L"),
        ];

        for variant in variants {
            let fast32_decoded: [u8; N] = fast32::base32::CROCKFORD_LOWER
                .decode_str(&variant)
                .unwrap()
                .try_into()
                .unwrap();
            let ubid = Ubid::<N>::decode(&variant).unwrap();

            assert_eq!(input, fast32_decoded);
            assert_eq!(input, *ubid.as_ref());
        }
    }

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
        fn encode_matches_fast32_40(input in any::<[u8; 5]>()) {
            let ubid = Ubid40::from(input);
            let encoded = ubid.encode();

            prop_assert_eq!(&encoded, &fast32::base32::CROCKFORD_LOWER.encode(&input));
            prop_assert_eq!(ubid, Ubid40::decode(&encoded).unwrap());
        }

        #[test]
        fn encode_matches_fast32_80(input in any::<[u8; 10]>()) {
            let ubid = Ubid80::from(input);
            let encoded = ubid.encode();

            prop_assert_eq!(&encoded, &fast32::base32::CROCKFORD_LOWER.encode(&input));
            prop_assert_eq!(ubid, Ubid80::decode(&encoded).unwrap());
        }

        #[test]
        fn encode_matches_fast32_120(input in any::<[u8; 15]>()) {
            let ubid = Ubid120::from(input);
            let encoded = ubid.encode();

            prop_assert_eq!(&encoded, &fast32::base32::CROCKFORD_LOWER.encode(&input));
            prop_assert_eq!(ubid, Ubid120::decode(&encoded).unwrap());
        }

        #[test]
        fn encode_matches_fast32_160(input in any::<[u8; 20]>()) {
            let ubid = Ubid160::from(input);
            let encoded = ubid.encode();

            prop_assert_eq!(&encoded, &fast32::base32::CROCKFORD_LOWER.encode(&input));
            prop_assert_eq!(ubid, Ubid160::decode(&encoded).unwrap());
        }
    );

    proptest!(
        #[test]
        fn decode_variants_match_fast32_40(input in any::<[u8; 5]>()) {
            assert_decode_variants_match_fast32(input);
        }

        #[test]
        fn decode_variants_match_fast32_80(input in any::<[u8; 10]>()) {
            assert_decode_variants_match_fast32(input);
        }

        #[test]
        fn decode_variants_match_fast32_120(input in any::<[u8; 15]>()) {
            assert_decode_variants_match_fast32(input);
        }

        #[test]
        fn decode_variants_match_fast32_160(input in any::<[u8; 20]>()) {
            assert_decode_variants_match_fast32(input);
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

    #[test]
    fn display_and_debug_match_canonical_encoding() {
        let ubid40 = Ubid40::from([0, 1, 2, 3, 4]);
        assert_eq!(ubid40.encode(), ubid40.to_string());

        let ubid80 = Ubid80::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(ubid80.encode(), ubid80.to_string());

        let ubid120 = Ubid120::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]);
        assert_eq!(ubid120.encode(), ubid120.to_string());

        let ubid160 = Ubid160::from([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
        ]);
        let encoded = ubid160.encode();
        assert_eq!(encoded, ubid160.to_string());
        assert_eq!(format!("Ubid160{{{encoded}}}"), format!("{ubid160:?}"));
    }

    #[test]
    fn decode_accepts_crockford_aliases() {
        assert_eq!(Ubid40::from([0; 5]), Ubid40::decode("oooooooo").unwrap());
        assert_eq!(
            Ubid40::decode("11111111").unwrap(),
            Ubid40::decode("iIlL1111").unwrap()
        );

        let ubid = Ubid80::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(ubid, Ubid80::decode(&ubid.encode().to_uppercase()).unwrap());
    }

    #[test]
    fn decode_reports_invalid_input() {
        assert_eq!(
            "Invalid length of 7",
            Ubid40::decode("0000000").unwrap_err().to_string()
        );
        assert_eq!(
            "Invalid char of 'u' at position 0",
            Ubid40::decode("u0000000").unwrap_err().to_string()
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let ubid = Ubid80::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let encoded = ubid.encode();

        let serialized = serde_json::to_string(&ubid).unwrap();
        assert_eq!(format!("\"{encoded}\""), serialized);

        let deserialized: Ubid80 = serde_json::from_str(&serialized).unwrap();
        assert_eq!(ubid, deserialized);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_rejects_invalid_text() {
        assert!(serde_json::from_str::<Ubid80>("\"not-a-ubid\"").is_err());
    }

    #[cfg(feature = "proptest")]
    proptest!(
        #[test]
        fn arbitrary_roundtrip(ubid in any::<Ubid160>()) {
            let decoded: Ubid160 = ubid.to_string().parse().unwrap();
            prop_assert_eq!(ubid, decoded);
        }
    );
}
