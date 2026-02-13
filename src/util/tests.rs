use crate::util::{expand_upce_to_upca, extract_digits, validate_gtin};

#[test]
fn expand_upce() {
    let cases = vec![("04182634", "041800000265"), ("0 123450 3", "012000003455")];

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
        ("8595701 530526", true),
        ("8595701 542376", true),
        ("8 595682 148871", true),
        ("8595701 542377", false),
        ("0 71720 53977 4", true),
        ("0 41420 06785 3", true),
        ("0 71720 53977 5", false),
        ("5201 3485", true),
        ("5201 3486", false),
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
        "8595701-530526",
        "8595701 542376",
        "8:595682:148871",
        "0h71720 53977 4",
        "0 41420_06785_3",
        "5201 3485",
    ];

    for gtin_str in cases {
        let gtin = crate::GTIN::try_from(gtin_str);
        assert!(gtin.is_ok(), "Failed to parse GTIN: {}", gtin_str);
    }
}
