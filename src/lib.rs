//! A library for parsing, validating, and working with GTIN (Global Trade Item Number) barcodes.
//!
//! Supports UPC-A, UPC-E, EAN-8, EAN-13, and GTIN-14 formats.
//!
//! Optional features:
//!
//! - `random`: random GTIN generation with valid checksums.
//! - `serde`: JSON-friendly serialization and deserialization support.
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
use rand::Rng;
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

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut digits = util::extract_digits(value);

        if !util::validate_gtin(&digits) {
            if digits.len() < 8 || digits.len() > 14 {
                return Err(GtinError::InvalidLength(digits.len()));
            }
            return Err(GtinError::InvalidChecksum);
        }

        match digits.len() {
            8 => {
                // UPC-E always starts with 0 (number system digit).
                // If the first digit is non-zero, this must be an EAN-8.
                if digits[0] != 0 {
                    Ok(GTIN::Ean8(digits.try_into().unwrap()))
                } else {
                    Ok(GTIN::UpcE(digits.try_into().unwrap()))
                }
            }
            // 11 digits is likely a UPC-A with a leading zero stripped by another system
            11 => {
                digits.insert(0, 0);
                Ok(GTIN::UpcA(digits.try_into().unwrap()))
            }
            12 => Ok(GTIN::UpcA(digits.try_into().unwrap())),
            // EAN-13 with a leading 0 is equivalent to a UPC-A; prefer the
            // more specific representation so round-tripping through databases
            // that zero-pad UPC-A codes recovers the original format.
            13 if digits[0] == 0 => Ok(GTIN::UpcA(digits[1..].try_into().unwrap())),
            13 => Ok(GTIN::Ean13(digits.try_into().unwrap())),
            14 => Ok(GTIN::Gtin14(digits.try_into().unwrap())),
            n => Err(GtinError::InvalidLength(n)),
        }
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
        let mut rng = rand::thread_rng();
        Self::random_with_rng(&mut rng)
    }

    /// Generates a random GTIN of the requested type.
    ///
    /// The returned GTIN always contains a valid checksum digit.
    /// EAN-8 and EAN-13 values use a non-zero leading digit so they round-trip
    /// through this crate's automatic format detection as the requested type.
    #[cfg(feature = "random")]
    pub fn random_of_type(gtin_type: GtinType) -> Self {
        let mut rng = rand::thread_rng();
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
        let index = rng.gen_range(0..GtinType::ALL.len());
        Self::random_of_type_with_rng(GtinType::ALL[index], rng)
    }

    /// Generates a random GTIN of the requested type using the supplied random number generator.
    ///
    /// The returned GTIN always contains a valid checksum digit.
    /// EAN-8 and EAN-13 values use a non-zero leading digit so they round-trip
    /// through this crate's automatic format detection as the requested type.
    #[cfg(feature = "random")]
    pub fn random_of_type_with_rng<R>(gtin_type: GtinType, rng: &mut R) -> Self
    where
        R: Rng + ?Sized,
    {
        match gtin_type {
            GtinType::UpcE => GTIN::UpcE(random_gtin_digits(rng, FirstDigit::Fixed(0))),
            GtinType::UpcA => GTIN::UpcA(random_gtin_digits(rng, FirstDigit::Any)),
            GtinType::Ean8 => GTIN::Ean8(random_gtin_digits(rng, FirstDigit::NonZero)),
            GtinType::Ean13 => GTIN::Ean13(random_gtin_digits(rng, FirstDigit::NonZero)),
            GtinType::Gtin14 => GTIN::Gtin14(random_gtin_digits(rng, FirstDigit::Any)),
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
    pub fn parse_upce(input: &str) -> Result<Self, GtinError> {
        let digits = util::extract_digits(input);
        if digits.len() != 8 {
            return Err(GtinError::InvalidLength(digits.len()));
        }
        if !util::validate_gtin(&digits) {
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
    Fixed(u8),
}

#[cfg(feature = "random")]
fn random_gtin_digits<const N: usize, R>(rng: &mut R, first_digit: FirstDigit) -> [u8; N]
where
    R: Rng + ?Sized,
{
    let mut digits = [0u8; N];

    digits[0] = match first_digit {
        FirstDigit::Any => rng.gen_range(0..=9),
        FirstDigit::NonZero => rng.gen_range(1..=9),
        FirstDigit::Fixed(digit) => digit,
    };
    for digit in &mut digits[1..N - 1] {
        *digit = rng.gen_range(0..=9);
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
