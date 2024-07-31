use std::fmt::{Display, Formatter};

use util::{digits_to_string, validate_gtin};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod util;

/// An enum to hold GTIN variants
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GTIN {
    UpcE([u8; 8]),    // UPC-E always has 8 digits
    UpcA([u8; 12]),   // UPC-A always has 12 digits
    Ean8([u8; 8]),    // EAN-8 always has 8 digits
    Ean13([u8; 13]),  // EAN-13 always has 13 digits
    Gtin14([u8; 14]), // GTIN-14 always has 14 digits
}

impl Display for GTIN {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            GTIN::UpcE(digits) => write!(f, "UPC-E: {}", digits_to_string(&digits)),
            GTIN::UpcA(digits) => write!(f, "UPC-A: {}", digits_to_string(&digits)),
            GTIN::Ean8(digits) => write!(f, "EAN-8: {}", digits_to_string(&digits)),
            GTIN::Ean13(digits) => write!(f, "EAN-13: {}", digits_to_string(&digits)),
            GTIN::Gtin14(digits) => write!(f, "GTIN-14: {}", digits_to_string(&digits)),
        }
    }
}

impl std::convert::TryFrom<&str> for GTIN {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut digits: Vec<u8> = util::extract_digits(value);

        if validate_gtin(&digits) {
            match digits.len() {
                8 => {
                    // Try to determine if it is UPC-E or EAN-8
                    // Simple heuristic: UPC-E is mostly used in North America and rarely has leading zeroes.
                    if digits[0] == 0 {
                        Ok(GTIN::Ean8(digits.try_into().map_err(|_| {
                            "digits vector does not have exactly 8 elements".to_string()
                        })?))
                    } else {
                        Ok(GTIN::UpcE(digits.try_into().map_err(|_| {
                            "digits vector does not have exactly 8 elements".to_string()
                        })?))
                    }
                }
                // 11 digits is probably a UPC-A with a leading zero that was removed
                // when the data was stored as a number in another system
                11 => {
                    digits.insert(0, 0);
                    Ok(GTIN::UpcA(digits.try_into().map_err(|_| {
                        "digits vector does not have exactly 12 elements".to_string()
                    })?))
                }
                12 => Ok(GTIN::UpcA(digits.try_into().map_err(|_| {
                    "digits vector does not have exactly 12 elements".to_string()
                })?)),
                13 => Ok(GTIN::Ean13(digits.try_into().map_err(|_| {
                    "digits vector does not have exactly 13 elements".to_string()
                })?)),
                14 => Ok(GTIN::Gtin14(digits.try_into().map_err(|_| {
                    "digits vector does not have exactly 14 elements".to_string()
                })?)),
                _ => Err("Unsupported GTIN length".to_string()),
            }
        } else {
            Err("Invalid GTIN checksum".to_string())
        }
    }
}

impl GTIN {
    pub fn digits(&self) -> &[u8] {
        match self {
            GTIN::UpcE(digits) => digits,
            GTIN::UpcA(digits) => digits,
            GTIN::Ean8(digits) => digits,
            GTIN::Ean13(digits) => digits,
            GTIN::Gtin14(digits) => digits,
        }
    }

    pub fn as_ean13(self) -> Option<GTIN> {
        match self {
            GTIN::Ean13(_) => Some(self),
            GTIN::UpcA(digits) => {
                let mut ean13_digits = [0; 13]; // Initialize all elements to zero
                ean13_digits[1..13].copy_from_slice(&digits[0..12]); // Copy UPC-A digits, including the check digit
                Some(GTIN::Ean13(ean13_digits))
            }
            _ => None, // For other GTIN types, we return None TODO: Implement conversion for other GTIN types
        }
    }

    pub fn country_code(&self) -> Option<&'static str> {
        // TODO: implement strong types? https://github.com/rust-iso/rust_iso3166
        match self.number_system() {
            // Check special conditions for non-general number systems
            NumberSystem::Drug => Some("US"), // US drug or supplement
            // Check special conditions for non-general number systems
            NumberSystem::StoreUse
            | NumberSystem::Coupon
            | NumberSystem::Isbn
            | NumberSystem::Issn
            | NumberSystem::Refund => None, // No country for these codes
            _ => {
                let prefix = self
                    .as_ean13()?
                    .digits()
                    .iter()
                    .take(3)
                    .fold(0, |acc, &digit| acc * 10 + (digit as usize));

                match prefix {
                    0..=139 => Some("US"),
                    300..=379 => Some("FR"), // France
                    380 => Some("BG"),
                    383 => Some("SI"),
                    385 => Some("HR"),
                    387 => Some("BA"),
                    389 => Some("ME"),
                    390 => Some("KOSOVO"), // or appropriate ISO code
                    400..=440 => Some("DE"),
                    450..=459 | 490..=499 => Some("JP"),
                    460..=469 => Some("RU"),
                    470 => Some("KG"),
                    471 => Some("TW"),
                    474 => Some("EE"),
                    500..=509 => Some("GB"),
                    520..=521 => Some("GR"),
                    539 => Some("IE"),
                    540..=549 => Some("BE"), // Belgium & Luxembourg
                    570..=579 => Some("DK"),
                    590 => Some("PL"),
                    599 => Some("HU"),
                    618 => Some("CI"),       // Ivory Coast
                    619 => Some("TN"),       // Tunisia
                    640..=649 => Some("FI"), // Finland
                    700..=709 => Some("NO"),
                    730..=739 => Some("SE"), // Sweden
                    742 => Some("HN"),       // Honduras
                    750 => Some("MX"),       // Mexico
                    754..=755 => Some("CA"),
                    759 => Some("VE"),
                    760..=769 => Some("CH"), // Switzerland
                    773 => Some("UY"),       // Uruguay
                    789..=790 => Some("BR"), // Brazil
                    800..=839 => Some("IT"), // Italy
                    840..=849 => Some("ES"), // Spain
                    858 => Some("SK"),       // Slovakia
                    859 => Some("CZ"),       // Czech Republic
                    860 => Some("RS"),
                    870..=879 => Some("NL"), // Netherlands
                    888 => Some("SG"),
                    885 => Some("TH"),       // Thailand
                    900..=919 => Some("AT"), // Austria
                    930..=939 => Some("AU"), // Australia
                    940..=949 => Some("NZ"), // New Zealand
                    _ => None,
                }
            }
        }
    }

    pub fn number_system(&self) -> NumberSystem {
        match self.as_ean13() {
            Some(gtin) => NumberSystem::from_ean13_prefix(&gtin.digits()[0..3]),
            None => NumberSystem::Unknown,
        }
    }
}

impl Serialize for GTIN {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match *self {
            GTIN::UpcE(digits) | GTIN::Ean8(digits) => digits_to_string(&digits),
            GTIN::UpcA(digits) => digits_to_string(&digits),
            GTIN::Ean13(digits) => digits_to_string(&digits),
            GTIN::Gtin14(digits) => digits_to_string(&digits),
        };
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for GTIN {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        GTIN::try_from(s.as_str()).map_err(serde::de::Error::custom)
    }
}

// TODO: Add tests for all number systems
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
    pub fn from_ean13_prefix(prefix: &[u8]) -> Self {
        if prefix.len() != 3 {
            return NumberSystem::Unknown; // Ensure we have exactly three digits
        }

        // Calculate the numeric value of the prefix assuming each element in the array is a single decimal digit
        let number = (prefix[0] as usize) * 100 + (prefix[1] as usize) * 10 + (prefix[2] as usize);

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
pub mod tests;
