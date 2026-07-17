//! A library for parsing, validating, and working with GTIN (Global Trade Item Number) barcodes.
//!
//! Supports UPC-A, UPC-E, EAN-8, EAN-13, and GTIN-14 formats.
//!
//! Optional features:
//!
//! - `random`: random GTIN generation with valid checksums.
//! - `serde`: JSON-friendly serialization and deserialization support.
//! - `sqlx`: PostgreSQL encode/decode support for SQLx.
//!
//! # Examples
//!
//! ```
//! use gtin::GTIN;
//!
//! let barcode: GTIN = "071720539774".parse().unwrap();
//! assert_eq!(barcode.format_name(), "UPC-A");
//! assert_eq!(barcode.country_code(), Some("US"));
//! ```

use std::fmt::{Display, Formatter};

#[cfg(feature = "random")]
use rand::{Rng, RngExt};
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod util;

/// Error type for GTIN parsing and conversion operations.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum GtinError {
    /// The input has an unsupported number of digits.
    InvalidLength(usize),
    /// The check digit does not match the calculated checksum.
    InvalidChecksum,
    /// A conversion between GTIN formats failed.
    ConversionFailed,
}

impl Display for GtinError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GtinError::InvalidLength(len) => write!(f, "unsupported GTIN length: {len}"),
            GtinError::InvalidChecksum => write!(f, "invalid GTIN checksum"),
            GtinError::ConversionFailed => write!(f, "GTIN conversion failed"),
        }
    }
}

impl std::error::Error for GtinError {}

/// A GTIN (Global Trade Item Number) barcode.
///
/// Each variant stores its digits as a fixed-size byte array where each element
/// is a single decimal digit (0-9).
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum GTIN {
    UpcE([u8; 8]),
    UpcA([u8; 12]),
    Ean8([u8; 8]),
    Ean13([u8; 13]),
    Gtin14([u8; 14]),
}

/// A supported GTIN type/format.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum GtinType {
    UpcE,
    UpcA,
    Ean8,
    Ean13,
    Gtin14,
}

impl GtinType {
    /// All GTIN types supported by this crate.
    pub const ALL: [Self; 5] = [
        Self::UpcE,
        Self::UpcA,
        Self::Ean8,
        Self::Ean13,
        Self::Gtin14,
    ];

    /// Returns the number of digits used by this GTIN type.
    pub const fn digit_count(self) -> usize {
        match self {
            Self::UpcE | Self::Ean8 => 8,
            Self::UpcA => 12,
            Self::Ean13 => 13,
            Self::Gtin14 => 14,
        }
    }

    /// Returns the display name of this GTIN type (e.g., "UPC-A", "EAN-13").
    pub const fn format_name(self) -> &'static str {
        match self {
            Self::UpcE => "UPC-E",
            Self::UpcA => "UPC-A",
            Self::Ean8 => "EAN-8",
            Self::Ean13 => "EAN-13",
            Self::Gtin14 => "GTIN-14",
        }
    }
}

impl Display for GTIN {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", util::digits_to_string(self.digits()))
    }
}

impl std::str::FromStr for GTIN {
    type Err = GtinError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<&str> for GTIN {
    type Error = GtinError;

    /// Parses a GTIN, normalizing expanded short-format codes to their
    /// canonical 8-digit form.
    ///
    /// A code whose digits are all zero except the trailing eight is
    /// returned as EAN-8, and a UPC-A matching a GS1 zero-suppression
    /// pattern is returned as UPC-E. When a code qualifies for both, the
    /// EAN-8 interpretation wins: GS1 reserves the leading-zeros space for
    /// GTIN-8. Use [`GTIN::as_upca`] or [`GTIN::as_ean13`] to expand the
    /// result where a longer form is needed.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut digits = util::extract_digits(value);

        if digits.len() < 8 || digits.len() > 14 {
            return Err(GtinError::InvalidLength(digits.len()));
        }

        // UPC-E uses a different check-digit rule than the other formats:
        // its check digit is the check digit of the expanded UPC-A. Try it
        // before the standard checksum. UPC-E always starts with 0 (number
        // system digit); an 8-digit code that fails UPC-E validation may
        // still be a valid EAN-8.
        if util::validate_upce(&digits) {
            return Ok(GTIN::UpcE(digits.try_into().unwrap()));
        }

        if !util::validate_gtin(&digits) {
            return Err(GtinError::InvalidChecksum);
        }

        let gtin = match digits.len() {
            8 => GTIN::Ean8(digits.try_into().unwrap()),
            // 11 digits is likely a UPC-A with a leading zero stripped by another system
            11 => {
                digits.insert(0, 0);
                GTIN::UpcA(digits.try_into().unwrap())
            }
            12 => GTIN::UpcA(digits.try_into().unwrap()),
            // EAN-13 with a leading 0 is equivalent to a UPC-A; prefer the
            // more specific representation so round-tripping through databases
            // that zero-pad UPC-A codes recovers the original format.
            13 if digits[0] == 0 => GTIN::UpcA(digits[1..].try_into().unwrap()),
            13 => GTIN::Ean13(digits.try_into().unwrap()),
            14 => GTIN::Gtin14(digits.try_into().unwrap()),
            n => return Err(GtinError::InvalidLength(n)),
        };

        // Normalize expanded short-format codes to their canonical 8-digit
        // form, trying zero-padding before UPC-E suppression so the EAN-8
        // interpretation wins when a code qualifies for both.
        Ok(gtin.as_ean8().or_else(|| gtin.as_upce()).unwrap_or(gtin))
    }
}

impl GTIN {
    /// Returns the raw digits of this GTIN as a byte slice.
    pub fn digits(&self) -> &[u8] {
        match self {
            GTIN::UpcE(d) => d,
            GTIN::UpcA(d) => d,
            GTIN::Ean8(d) => d,
            GTIN::Ean13(d) => d,
            GTIN::Gtin14(d) => d,
        }
    }

    /// Returns the specific GTIN type/format represented by this value.
    pub fn gtin_type(&self) -> GtinType {
        match self {
            GTIN::UpcE(_) => GtinType::UpcE,
            GTIN::UpcA(_) => GtinType::UpcA,
            GTIN::Ean8(_) => GtinType::Ean8,
            GTIN::Ean13(_) => GtinType::Ean13,
            GTIN::Gtin14(_) => GtinType::Gtin14,
        }
    }

    /// Returns the name of this GTIN format (e.g., "UPC-A", "EAN-13").
    pub fn format_name(&self) -> &'static str {
        self.gtin_type().format_name()
    }

    /// Returns the number of digits in this GTIN. Always 8-14.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.digits().len()
    }

    /// Generates a random GTIN using one of the supported types.
    ///
    /// The returned GTIN always contains a valid checksum digit.
    #[cfg(feature = "random")]
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self::random_with_rng(&mut rng)
    }

    /// Generates a random GTIN of the requested type.
    ///
    /// The returned GTIN always contains a valid checksum digit.
    /// EAN-8 and EAN-13 values use a non-zero leading digit, and UPC-A and
    /// GTIN-14 values avoid the expanded short-format patterns that parsing
    /// normalizes, so all results round-trip through this crate's automatic
    /// format detection as the requested type.
    #[cfg(feature = "random")]
    pub fn random_of_type(gtin_type: GtinType) -> Self {
        let mut rng = rand::rng();
        Self::random_of_type_with_rng(gtin_type, &mut rng)
    }

    /// Generates a random GTIN using the supplied random number generator.
    ///
    /// The returned GTIN always contains a valid checksum digit.
    #[cfg(feature = "random")]
    pub fn random_with_rng<R>(rng: &mut R) -> Self
    where
        R: Rng + ?Sized,
    {
        let index = rng.random_range(0..GtinType::ALL.len());
        Self::random_of_type_with_rng(GtinType::ALL[index], rng)
    }

    /// Generates a random GTIN of the requested type using the supplied random number generator.
    ///
    /// The returned GTIN always contains a valid checksum digit.
    /// EAN-8 and EAN-13 values use a non-zero leading digit, and UPC-A and
    /// GTIN-14 values avoid the expanded short-format patterns that parsing
    /// normalizes, so all results round-trip through this crate's automatic
    /// format detection as the requested type.
    #[cfg(feature = "random")]
    pub fn random_of_type_with_rng<R>(gtin_type: GtinType, rng: &mut R) -> Self
    where
        R: Rng + ?Sized,
    {
        match gtin_type {
            GtinType::UpcE => GTIN::UpcE(random_upce_digits(rng)),
            GtinType::UpcA => loop {
                let gtin = GTIN::UpcA(random_gtin_digits(rng, FirstDigit::Any));
                if gtin.as_upce().is_none() && gtin.as_ean8().is_none() {
                    break gtin;
                }
            },
            GtinType::Ean8 => GTIN::Ean8(random_gtin_digits(rng, FirstDigit::NonZero)),
            GtinType::Ean13 => GTIN::Ean13(random_gtin_digits(rng, FirstDigit::NonZero)),
            GtinType::Gtin14 => loop {
                let gtin = GTIN::Gtin14(random_gtin_digits(rng, FirstDigit::Any));
                if gtin.as_ean8().is_none() {
                    break gtin;
                }
            },
        }
    }

    /// Parses an 8-digit input explicitly as EAN-8, bypassing the UPC-E/EAN-8 heuristic.
    pub fn parse_ean8(input: &str) -> Result<Self, GtinError> {
        let digits = util::extract_digits(input);
        if digits.len() != 8 {
            return Err(GtinError::InvalidLength(digits.len()));
        }
        if !util::validate_gtin(&digits) {
            return Err(GtinError::InvalidChecksum);
        }
        Ok(GTIN::Ean8(digits.try_into().unwrap()))
    }

    /// Parses an 8-digit input explicitly as UPC-E, bypassing the UPC-E/EAN-8 heuristic.
    ///
    /// The check digit is validated against the expanded UPC-A equivalent,
    /// so the input must be a structurally valid UPC-E (leading number
    /// system digit 0).
    pub fn parse_upce(input: &str) -> Result<Self, GtinError> {
        let digits = util::extract_digits(input);
        if digits.len() != 8 {
            return Err(GtinError::InvalidLength(digits.len()));
        }
        if !util::validate_upce(&digits) {
            return Err(GtinError::InvalidChecksum);
        }
        Ok(GTIN::UpcE(digits.try_into().unwrap()))
    }

    /// Converts this GTIN to an EAN-13 representation, if possible.
    ///
    /// Returns `Some` for UPC-A, UPC-E, and EAN-13. Returns `None` for EAN-8
    /// and GTIN-14, which have different structures that don't map directly to EAN-13.
    pub fn as_ean13(self) -> Option<GTIN> {
        match self {
            GTIN::Ean13(_) => Some(self),
            GTIN::UpcA(digits) => {
                let mut ean13 = [0u8; 13];
                ean13[1..13].copy_from_slice(&digits);
                Some(GTIN::Ean13(ean13))
            }
            GTIN::UpcE(digits) => {
                let upca = util::expand_upce_to_upca(&digits).ok()?;
                let mut ean13 = [0u8; 13];
                ean13[1..13].copy_from_slice(upca.digits());
                Some(GTIN::Ean13(ean13))
            }
            _ => None,
        }
    }

    /// Converts this GTIN to a UPC-A representation, if possible.
    ///
    /// Returns `Some` for UPC-A and UPC-E. Returns `None` for EAN-8, EAN-13,
    /// and GTIN-14, which have different structures that don't map directly to UPC-A.
    pub fn as_upca(self) -> Option<GTIN> {
        match self {
            GTIN::UpcA(_) => Some(self),
            GTIN::UpcE(digits) => util::expand_upce_to_upca(&digits).ok(),
            _ => None,
        }
    }

    /// Converts this GTIN to its zero-suppressed UPC-E representation, if possible.
    ///
    /// Returns `Some` for UPC-E and for UPC-A codes whose digits match one of
    /// the GS1 zero-suppression patterns (number system 0 plus the required
    /// zeros in the manufacturer and item numbers). Returns `None` for
    /// UPC-A codes that cannot be suppressed and for all other formats.
    ///
    /// This is the inverse of [`GTIN::as_upca`]: a UPC-E that another system
    /// stored in its expanded UPC-A form is recovered exactly.
    pub fn as_upce(self) -> Option<GTIN> {
        match self {
            GTIN::UpcE(_) => Some(self),
            GTIN::UpcA(digits) => util::compress_upca_to_upce(&digits).ok(),
            _ => None,
        }
    }

    /// Converts this GTIN to an EAN-8 representation, if possible.
    ///
    /// EAN-8 codes are assigned from their own GS1 namespace rather than
    /// derived from a longer code, so the only conversion this performs is
    /// recovering an EAN-8 that another system stored zero-padded to 12, 13,
    /// or 14 digits. Returns `Some` for EAN-8 itself and for longer codes
    /// that are all zeros except the trailing 8 digits (with a non-zero
    /// leading EAN-8 digit, matching this crate's format-detection
    /// heuristic). Returns `None` otherwise.
    ///
    /// Stripping leading zeros preserves the check digit, so the result is
    /// always a valid EAN-8 when the input checksum was valid.
    pub fn as_ean8(self) -> Option<GTIN> {
        match self {
            GTIN::Ean8(_) => Some(self),
            GTIN::UpcA(d) => Self::unpad_ean8(&d),
            GTIN::Ean13(d) => Self::unpad_ean8(&d),
            GTIN::Gtin14(d) => Self::unpad_ean8(&d),
            GTIN::UpcE(_) => None,
        }
    }

    /// Recovers a zero-padded EAN-8 from the trailing 8 digits of a longer code.
    fn unpad_ean8(digits: &[u8]) -> Option<GTIN> {
        let (padding, ean8) = digits.split_at(digits.len() - 8);
        if padding.iter().all(|&d| d == 0) && ean8[0] != 0 {
            Some(GTIN::Ean8(ean8.try_into().unwrap()))
        } else {
            None
        }
    }

    /// Returns the 3-digit GS1 prefix used for country and number system identification.
    fn gs1_prefix(&self) -> Option<[u8; 3]> {
        match self {
            GTIN::Ean13(d) => Some([d[0], d[1], d[2]]),
            GTIN::UpcA(d) => Some([0, d[0], d[1]]),
            GTIN::UpcE(_) => {
                let upca = util::expand_upce_to_upca(self.digits()).ok()?;
                let d = upca.digits();
                Some([0, d[0], d[1]])
            }
            GTIN::Ean8(d) => Some([d[0], d[1], d[2]]),
            GTIN::Gtin14(d) => Some([d[1], d[2], d[3]]),
        }
    }

    /// Returns the ISO 3166-1 alpha-2 country code for this GTIN, if determinable.
    /// A full list of the country code ranges can be found at: https://en.wikipedia.org/wiki/List_of_GS1_country_codes
    pub fn country_code(&self) -> Option<&'static str> {
        match self.number_system() {
            NumberSystem::Drug => Some("US"),
            NumberSystem::StoreUse
            | NumberSystem::Coupon
            | NumberSystem::Isbn
            | NumberSystem::Issn
            | NumberSystem::Refund => None,
            _ => {
                let prefix = self.gs1_prefix()?;
                let number =
                    (prefix[0] as usize) * 100 + (prefix[1] as usize) * 10 + prefix[2] as usize;

                match number {
                    0..=139 => Some("US"),
                    300..=379 => Some("FR"),
                    380 => Some("BG"),
                    383 => Some("SI"),
                    385 => Some("HR"),
                    387 => Some("BA"),
                    389 => Some("ME"),
                    390 => Some("KOSOVO"),
                    400..=440 => Some("DE"),
                    450..=459 | 490..=499 => Some("JP"),
                    460..=469 => Some("RU"),
                    470 => Some("KG"),
                    471 => Some("TW"),
                    474 => Some("EE"),
                    500..=509 => Some("GB"),
                    520..=521 => Some("GR"),
                    539 => Some("IE"),
                    540..=549 => Some("BE"),
                    570..=579 => Some("DK"),
                    590 => Some("PL"),
                    599 => Some("HU"),
                    618 => Some("CI"),
                    619 => Some("TN"),
                    640..=649 => Some("FI"),
                    700..=709 => Some("NO"),
                    730..=739 => Some("SE"),
                    742 => Some("HN"),
                    750 => Some("MX"),
                    754..=755 => Some("CA"),
                    759 => Some("VE"),
                    760..=769 => Some("CH"),
                    773 => Some("UY"),
                    789..=790 => Some("BR"),
                    800..=839 => Some("IT"),
                    840..=849 => Some("ES"),
                    858 => Some("SK"),
                    859 => Some("CZ"),
                    860 => Some("RS"),
                    870..=879 => Some("NL"),
                    885 => Some("TH"),
                    888 => Some("SG"),
                    900..=919 => Some("AT"),
                    930..=939 => Some("AU"),
                    940..=949 => Some("NZ"),
                    _ => None,
                }
            }
        }
    }

    /// Returns the number system classification for this GTIN.
    pub fn number_system(&self) -> NumberSystem {
        match self.gs1_prefix() {
            Some(prefix) => NumberSystem::from_prefix(&prefix),
            None => NumberSystem::Unknown,
        }
    }
}

#[cfg(feature = "random")]
enum FirstDigit {
    Any,
    NonZero,
}

/// Generates random UPC-E digits: number system 0, six random body digits,
/// and a check digit taken from the expanded UPC-A equivalent.
#[cfg(feature = "random")]
fn random_upce_digits<R>(rng: &mut R) -> [u8; 8]
where
    R: Rng + ?Sized,
{
    let mut digits = [0u8; 8];
    for digit in &mut digits[1..7] {
        *digit = rng.random_range(0..=9);
    }
    let upca = util::expand_upce_to_upca(&digits[1..7])
        .expect("6-digit UPC-E body always expands to a UPC-A");
    digits[7] = upca.digits()[11];
    digits
}

#[cfg(feature = "random")]
fn random_gtin_digits<const N: usize, R>(rng: &mut R, first_digit: FirstDigit) -> [u8; N]
where
    R: Rng + ?Sized,
{
    let mut digits = [0u8; N];

    digits[0] = match first_digit {
        FirstDigit::Any => rng.random_range(0..=9),
        FirstDigit::NonZero => rng.random_range(1..=9),
    };
    for digit in &mut digits[1..N - 1] {
        *digit = rng.random_range(0..=9);
    }
    digits[N - 1] = util::calculate_checksum_digit(&digits[..N - 1]);

    digits
}

#[cfg(feature = "serde")]
impl Serialize for GTIN {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&util::digits_to_string(self.digits()))
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for GTIN {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        GTIN::try_from(s.as_str()).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "sqlx")]
impl sqlx::Type<sqlx::Postgres> for GTIN {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }

    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::Postgres>>::compatible(ty)
    }
}

#[cfg(feature = "sqlx")]
impl sqlx::postgres::types::PgHasArrayType for GTIN {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::postgres::types::PgHasArrayType>::array_type_info()
    }

    fn array_compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        <String as sqlx::postgres::types::PgHasArrayType>::array_compatible(ty)
    }
}

#[cfg(feature = "sqlx")]
impl<'q> sqlx::Encode<'q, sqlx::Postgres> for GTIN {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Postgres as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        <String as sqlx::Encode<'q, sqlx::Postgres>>::encode(
            util::digits_to_string(self.digits()),
            buf,
        )
    }

    fn produces(&self) -> Option<sqlx::postgres::PgTypeInfo> {
        Some(<Self as sqlx::Type<sqlx::Postgres>>::type_info())
    }

    fn size_hint(&self) -> usize {
        self.len()
    }
}

#[cfg(feature = "sqlx")]
impl<'r> sqlx::Decode<'r, sqlx::Postgres> for GTIN {
    fn decode(
        value: <sqlx::Postgres as sqlx::Database>::ValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let value = <&str as sqlx::Decode<'r, sqlx::Postgres>>::decode(value)?;
        Self::try_from(value).map_err(Into::into)
    }
}

/// Classification of a GTIN's number system based on its GS1 prefix.
#[derive(Debug, PartialEq, Eq)]
pub enum NumberSystem {
    General,
    StoreUse,
    Coupon,
    Drug,
    Issn,
    Isbn,
    Refund,
    Unknown,
}

impl NumberSystem {
    /// Determines the number system from a 3-digit GS1 prefix.
    pub fn from_prefix(prefix: &[u8]) -> Self {
        if prefix.len() != 3 {
            return NumberSystem::Unknown;
        }

        let number = (prefix[0] as usize) * 100 + (prefix[1] as usize) * 10 + prefix[2] as usize;

        match number {
            20..=29 | 40..=49 | 200..=299 => NumberSystem::StoreUse,
            30..=39 => NumberSystem::Drug,
            50..=59 | 981..=984 | 990..=999 => NumberSystem::Coupon,
            977 => NumberSystem::Issn,
            978..=979 => NumberSystem::Isbn,
            980 => NumberSystem::Refund,
            _ => NumberSystem::General,
        }
    }
}

#[cfg(test)]
mod tests;
