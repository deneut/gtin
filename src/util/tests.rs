use crate::util::validate_gtin;

use super::expand_upce_to_upca;
use super::extract_digits;

#[test]
fn expand_upce() {
    let cases = vec![
        ("04182635", "041800000265"),
        ("0 123450 5", "0 12000 00345 5")
    ];

    for (upce_str, expected_upca_str) in cases {
        let upce_digits = extract_digits(upce_str);
        let upca_digits = extract_digits(expected_upca_str);

        match expand_upce_to_upca(&upce_digits) {
            Ok(result) => assert_eq!(
                result.digits(),
                upca_digits,
                "Failed to match UPC-E: {}",
                upce_str
            ),
            Err(e) => panic!("Failed to expand UPC-E {}: {}", upce_str, e),
        }
    }
}

#[test]
fn validate_digits() {
    let cases = vec![
        ("8595701 530526", true),   // EAN-13
        ("8595701 542376", true),   // EAN-13
        ("8 595682 148871", true),  // EAN-13
        ("8595701 542377", false),  // invalid EAN-13
        ("0 71720 53977 4", true),  // UPC-A
        ("0 41420 06785 3", true),  // UPC-A
        ("0 71720 53977 5", false), // invalid UPC-A
        ("5201 3485", true),        // EAN-8
        ("5201 3486", false),       // invalid EAN-8
    ];

    for (gtin, validity) in cases {
        assert_eq!(
            validate_gtin(&extract_digits(gtin)),
            validity,
            "Failed to match GTIN: {}",
            gtin
        );
    }
}


#[test]
fn handle_non_digit_characters() {
    let cases = vec![
        "8595701-530526",   // EAN-13
        "8595701 542376",   // EAN-13
        "8:595682:148871",   // EAN-13
        "0h71720 53977 4",  // UPC-A
        "0 41420_06785_3",   // UPC-A
        "5201 3485"        // EAN-8
    ];

    for gtin_str in cases {
      let gtin = crate::GTIN::try_from(gtin_str);
        assert!(gtin.is_ok(), "Failed to parse GTIN: {}", gtin_str);
    }
}