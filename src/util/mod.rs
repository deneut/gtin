use crate::{GTIN, GtinError};

pub(crate) fn digits_to_string(digits: &[u8]) -> String {
    digits.iter().map(|&d| (d + b'0') as char).collect()
}

pub(crate) fn calculate_checksum_digit(digits: &[u8]) -> u8 {
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(index, &digit)| {
            if index % 2 == 0 {
                // Digit needs to be converted to u32 before multiplication to avoid overflow
                digit as u32 * 3
            } else {
                digit as u32
            }
        })
        .sum(); // This sum is now a u32 sum, which is less likely to overflow

    (10 - (sum % 10) as u8) % 10 // Convert back to u8 for final calculation
}

pub(crate) fn validate_gtin(digits: &[u8]) -> bool {
    if digits.len() < 8 || digits.len() > 14 {
        return false;
    }

    let checksum_index = digits.len() - 1;
    let checksum_digit = digits[checksum_index];
    checksum_digit == calculate_checksum_digit(&digits[..checksum_index])
}

/// Validates an 8-digit UPC-E code.
///
/// A UPC-E check digit is not a checksum over the 8 compressed digits;
/// it is the check digit of the expanded UPC-A equivalent.
pub(crate) fn validate_upce(digits: &[u8]) -> bool {
    if digits.len() != 8 || digits[0] != 0 {
        return false;
    }

    match expand_upce_to_upca(digits) {
        Ok(upca) => upca.digits()[11] == digits[7],
        Err(_) => false,
    }
}

pub(crate) fn extract_digits(input: &str) -> Vec<u8> {
    input
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).unwrap() as u8)
        .collect()
}

/// Expands a UPC-E code to its full UPC-A representation.
pub(crate) fn expand_upce_to_upca(upce: &[u8]) -> Result<GTIN, GtinError> {
    if upce.len() < 6 || upce.len() > 8 {
        return Err(GtinError::ConversionFailed);
    }

    // Extract middle digits based on length
    let middle_digits = match upce.len() {
        6 => upce,
        7 => &upce[..6],
        8 => &upce[1..7],
        _ => return Err(GtinError::ConversionFailed),
    };

    // Decode based on the last digit rules
    let (manufacturer_number, item_number) = match middle_digits[5] {
        0..=2 => (
            vec![middle_digits[0], middle_digits[1], middle_digits[5], 0, 0],
            vec![0, 0, middle_digits[2], middle_digits[3], middle_digits[4]],
        ),
        3 => (
            vec![middle_digits[0], middle_digits[1], middle_digits[2], 0, 0],
            vec![0, 0, 0, middle_digits[3], middle_digits[4]],
        ),
        4 => (
            vec![
                middle_digits[0],
                middle_digits[1],
                middle_digits[2],
                middle_digits[3],
                0,
            ],
            vec![0, 0, 0, 0, middle_digits[4]],
        ),
        _ => (
            vec![
                middle_digits[0],
                middle_digits[1],
                middle_digits[2],
                middle_digits[3],
                middle_digits[4],
            ],
            vec![0, 0, 0, 0, middle_digits[5]],
        ),
    };

    // Assemble the new UPC-A number
    let mut new_upca_digits = vec![0]; // Start with number system digit
    new_upca_digits.extend(manufacturer_number);
    new_upca_digits.extend(item_number);

    // Calculate the check digit
    let check_digit = calculate_checksum_digit(&new_upca_digits);
    new_upca_digits.push(check_digit);

    if new_upca_digits.len() != 12 {
        return Err(GtinError::ConversionFailed);
    }

    let mut result = [0u8; 12];
    result.copy_from_slice(&new_upca_digits[..12]);
    Ok(GTIN::UpcA(result))
}

/// Compresses a 12-digit UPC-A into its zero-suppressed UPC-E form.
///
/// Only number system 0 codes whose manufacturer and item digits match one
/// of the four GS1 zero-suppression patterns can be compressed. The patterns
/// are tried in standard order, so the result is the canonical UPC-E for
/// codes that could be suppressed more than one way. The UPC-E check digit
/// is the UPC-A check digit, so `expand_upce_to_upca` recovers the input
/// exactly.
pub(crate) fn compress_upca_to_upce(upca: &[u8]) -> Result<GTIN, GtinError> {
    if upca.len() != 12 || upca[0] != 0 {
        return Err(GtinError::ConversionFailed);
    }

    let (m, i) = (&upca[1..6], &upca[6..11]);

    let body: [u8; 6] = if m[3] == 0 && m[4] == 0 && m[2] <= 2 && i[..2] == [0, 0] {
        // Manufacturer ends 000, 100, or 200; item up to 999.
        [m[0], m[1], i[2], i[3], i[4], m[2]]
    } else if m[3] == 0 && m[4] == 0 && i[..3] == [0, 0, 0] {
        // Manufacturer ends 00; item up to 99.
        [m[0], m[1], m[2], i[3], i[4], 3]
    } else if m[4] == 0 && i[..4] == [0, 0, 0, 0] {
        // Manufacturer ends 0; item up to 9.
        [m[0], m[1], m[2], m[3], i[4], 4]
    } else if i[..4] == [0, 0, 0, 0] && i[4] >= 5 {
        // Any manufacturer; item 5 through 9.
        [m[0], m[1], m[2], m[3], m[4], i[4]]
    } else {
        return Err(GtinError::ConversionFailed);
    };

    let mut upce = [0u8; 8];
    upce[1..7].copy_from_slice(&body);
    upce[7] = upca[11];
    Ok(GTIN::UpcE(upce))
}

#[cfg(test)]
mod tests;
