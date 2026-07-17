use crate::util::{
    compress_upca_to_upce, expand_upce_to_upca, extract_digits, validate_gtin, validate_upce,
};

#[test]
fn expand_upce() {
    let cases = vec![
        ("04182634", "041800000265"),
        ("0 123450 3", "012000003455"),
        ("04940308", "049000004038"),
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
fn compress_upca() {
    let cases = vec![
        ("041800000265", "04182635"),
        ("012000003455", "01234505"),
        ("049000004038", "04940308"),
    ];

    for (upca_str, expected_upce_str) in cases {
        let upca_digits = extract_digits(upca_str);
        let upce_digits = extract_digits(expected_upce_str);

        match compress_upca_to_upce(&upca_digits) {
            Ok(result) => assert_eq!(
                result.digits(),
                upce_digits,
                "Failed to match UPC-A: {}",
                upca_str
            ),
            Err(e) => panic!("Failed to compress UPC-A {}: {}", upca_str, e),
        }
    }
}

#[test]
fn compress_rejects_unsuppressible_upca() {
    let cases = vec![
        // Item number digits are not zero where suppression requires it.
        "071720539774",
        // Number system digit is not 0.
        "141800000262",
    ];

    for upca in cases {
        assert!(
            compress_upca_to_upce(&extract_digits(upca)).is_err(),
            "Expected compression to fail for UPC-A: {}",
            upca
        );
    }
}

#[test]
fn compress_inverts_expand_for_all_upce_bodies() {
    // Every possible 6-digit UPC-E body must expand to a UPC-A that
    // compresses back to a UPC-E encoding the same UPC-A. When several
    // suppression patterns overlap, compression picks the canonical body,
    // which may differ from a non-canonical input, so compare through a
    // second expansion rather than digit-for-digit.
    for n in 0..1_000_000u32 {
        let mut body = [0u8; 6];
        let mut rest = n;
        for digit in body.iter_mut().rev() {
            *digit = (rest % 10) as u8;
            rest /= 10;
        }

        let upca = expand_upce_to_upca(&body).unwrap();
        let upce = compress_upca_to_upce(upca.digits())
            .unwrap_or_else(|e| panic!("Failed to compress expansion of body {n:06}: {e}"));
        let reexpanded = expand_upce_to_upca(upce.digits()).unwrap();
        assert_eq!(reexpanded, upca, "body: {n:06}");
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
fn validate_upce_digits() {
    let cases = vec![
        // Check digit matches the expanded UPC-A (049000004038).
        ("04940308", true),
        ("04182635", true),
        // Valid EAN-8 checksum, but not a valid UPC-E check digit.
        ("04182634", false),
        // Number system digit must be 0.
        ("52013485", false),
        // Wrong length.
        ("0494030", false),
    ];

    for (upce, validity) in cases {
        assert_eq!(
            validate_upce(&extract_digits(upce)),
            validity,
            "Failed to match UPC-E: {}",
            upce
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
