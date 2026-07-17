#[cfg(feature = "random")]
use rand::{SeedableRng, rngs::StdRng};

#[cfg(feature = "random")]
use crate::GtinType;
use crate::{GTIN, GtinError, NumberSystem};

#[test]
fn parse_formats() {
    let cases = vec![
        ("071720539774", "UPC-A"),
        ("0041303073414", "UPC-A"),
        ("04182635", "UPC-E"),
        ("04940308", "UPC-E"),
        // Valid as EAN-8 but not as UPC-E (its UPC-E check digit would be 5).
        ("04182634", "EAN-8"),
        // Valid under both check-digit rules; UPC-E takes precedence.
        ("01123456", "UPC-E"),
        ("52013485", "EAN-8"),
        ("8595701530526", "EAN-13"),
        ("00012345678905", "GTIN-14"),
    ];

    for (input, expected_format) in cases {
        let gtin = GTIN::try_from(input).unwrap();
        assert_eq!(gtin.format_name(), expected_format, "input: {input}");
    }
}

#[test]
fn determine_number_system() {
    let cases = vec![
        ("8595701 530526", NumberSystem::General),
        ("8595701 542376", NumberSystem::General),
        ("8 595682 148871", NumberSystem::General),
        ("0 71720 53977 4", NumberSystem::General),
        ("0 41420 06785 3", NumberSystem::General),
        ("5201 3485", NumberSystem::General),
        ("9783161484100", NumberSystem::Isbn),
        ("9772434561006", NumberSystem::Issn),
        ("02 45678 1 0543 9", NumberSystem::StoreUse),
    ];

    for (input, expected) in &cases {
        let gtin = GTIN::try_from(*input).unwrap();
        assert_eq!(gtin.number_system(), *expected, "input: {input}");
    }
}

#[test]
fn determine_country_code() {
    let cases = vec![
        ("8595701 530526", Some("CZ")),
        ("8595701 542376", Some("CZ")),
        ("8 595682 148871", Some("CZ")),
        ("8 410175 086501", Some("ES")),
        ("0 71720 53977 4", Some("US")),
        ("0 41420 06785 3", Some("US")),
        ("0 123450 3", Some("US")),
        ("5201 3485", Some("GR")),
        ("02 45678 1 0543 9", None),
    ];

    for (input, expected) in &cases {
        let gtin = GTIN::try_from(*input).unwrap();
        assert_eq!(gtin.country_code(), *expected, "input: {input}");
    }
}

#[test]
fn reject_invalid_checksum() {
    let result = GTIN::try_from("071720539775");
    assert_eq!(result, Err(GtinError::InvalidChecksum));
}

#[test]
fn reject_invalid_length() {
    let result = GTIN::try_from("12345");
    assert_eq!(result, Err(GtinError::InvalidLength(5)));
}

#[test]
fn display_outputs_digits_only() {
    let gtin = GTIN::UpcA([0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]);
    assert_eq!(gtin.to_string(), "071720539774");
}

#[test]
fn format_name() {
    assert_eq!(
        GTIN::UpcA([0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]).format_name(),
        "UPC-A"
    );
    assert_eq!(
        GTIN::Ean13([8, 5, 9, 5, 7, 0, 1, 5, 3, 0, 5, 2, 6]).format_name(),
        "EAN-13"
    );
}

#[test]
fn parse_from_str_trait() {
    let gtin: GTIN = "071720539774".parse().unwrap();
    assert_eq!(gtin, GTIN::UpcA([0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]));
}

#[test]
fn explicit_parse_ean8() {
    // "01123456" is valid under both check-digit rules, so try_from
    // classifies it as UPC-E, but parse_ean8 forces EAN-8.
    let gtin = GTIN::parse_ean8("01123456").unwrap();
    assert_eq!(gtin.format_name(), "EAN-8");
}

#[test]
fn explicit_parse_upce() {
    let gtin = GTIN::parse_upce("04940308").unwrap();
    assert_eq!(gtin.format_name(), "UPC-E");

    // Valid EAN-8, but its check digit doesn't match the expanded UPC-A.
    assert_eq!(
        GTIN::parse_upce("04182634"),
        Err(GtinError::InvalidChecksum)
    );
    // UPC-E must start with number system digit 0.
    assert_eq!(
        GTIN::parse_upce("52013485"),
        Err(GtinError::InvalidChecksum)
    );
}

#[test]
fn as_upce_recovers_expanded_upce() {
    // A UPC-E stored in its expanded UPC-A form compresses back exactly.
    let expanded = GTIN::try_from("041800000265").unwrap();
    assert_eq!(expanded.format_name(), "UPC-A");

    let upce = expanded.as_upce().unwrap();
    assert_eq!(upce, GTIN::try_from("04182635").unwrap());
    assert_eq!(upce.as_upca(), Some(expanded));

    // UPC-E passes through unchanged.
    assert_eq!(upce.as_upce(), Some(upce));
}

#[test]
fn as_upce_rejects_unsuppressible_codes() {
    // A UPC-A whose digits don't match any zero-suppression pattern.
    assert_eq!(GTIN::try_from("071720539774").unwrap().as_upce(), None);
    // Other formats never convert.
    assert_eq!(GTIN::try_from("52013485").unwrap().as_upce(), None);
    assert_eq!(GTIN::try_from("8595701530526").unwrap().as_upce(), None);
}

#[test]
fn as_ean8_recovers_zero_padded_ean8() {
    let ean8 = GTIN::try_from("52013485").unwrap();

    // Zero-padded to 12 and 13 digits both parse as UPC-A; 14 as GTIN-14.
    for padded in ["000052013485", "0000052013485", "00000052013485"] {
        let gtin = GTIN::try_from(padded).unwrap();
        assert_eq!(gtin.as_ean8(), Some(ean8), "input: {padded}");
    }

    // EAN-8 passes through unchanged.
    assert_eq!(ean8.as_ean8(), Some(ean8));
}

#[test]
fn as_ean8_rejects_non_padded_codes() {
    // Ordinary codes with real digits in the padding positions.
    assert_eq!(GTIN::try_from("071720539774").unwrap().as_ean8(), None);
    assert_eq!(GTIN::try_from("8595701530526").unwrap().as_ean8(), None);
    // Trailing 8 digits start with 0, so this is not treated as a padded
    // EAN-8, matching the crate's leading-digit format heuristic.
    assert_eq!(GTIN::try_from("0000004182634").unwrap().as_ean8(), None);
    // UPC-E never converts to EAN-8.
    assert_eq!(GTIN::try_from("04182635").unwrap().as_ean8(), None);
}

#[test]
fn len() {
    assert_eq!(GTIN::try_from("071720539774").unwrap().len(), 12);
    assert_eq!(GTIN::try_from("8595701530526").unwrap().len(), 13);
    assert_eq!(GTIN::try_from("52013485").unwrap().len(), 8);
}

#[test]
#[cfg(feature = "random")]
fn generate_random_gtins_of_requested_type() {
    let mut rng = StdRng::seed_from_u64(42);

    for gtin_type in GtinType::ALL {
        for _ in 0..50 {
            let gtin = GTIN::random_of_type_with_rng(gtin_type, &mut rng);
            let serialized = gtin.to_string();
            let reparsed = GTIN::try_from(serialized.as_str()).unwrap();

            assert_eq!(gtin.gtin_type(), gtin_type);
            assert_eq!(gtin.format_name(), gtin_type.format_name());
            assert_eq!(gtin.len(), gtin_type.digit_count());
            assert_eq!(reparsed, gtin);
        }
    }
}

#[test]
#[cfg(feature = "random")]
fn generate_random_gtins_with_random_type() {
    let mut rng = StdRng::seed_from_u64(123);

    for _ in 0..50 {
        let gtin = GTIN::random_with_rng(&mut rng);
        let serialized = gtin.to_string();

        assert!(GtinType::ALL.contains(&gtin.gtin_type()));
        assert_eq!(GTIN::try_from(serialized.as_str()).unwrap(), gtin);
    }
}

// serde tests

#[test]
#[cfg(feature = "serde")]
fn serialize_upca() {
    let gtin = GTIN::UpcA([0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]);
    let serialized = serde_json::to_string(&gtin).unwrap();
    assert_eq!(serialized, "\"071720539774\"");
}

#[test]
#[cfg(feature = "serde")]
fn deserialize_upca_with_spaces() {
    let data = "\"0 71720 53977 4\"";
    let deserialized: GTIN = serde_json::from_str(data).unwrap();
    match deserialized {
        GTIN::UpcA(digits) => assert_eq!(digits, [0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]),
        _ => panic!("Deserialized to incorrect type"),
    }
}

#[test]
#[cfg(feature = "serde")]
fn deserialize_upca_with_missing_initial_zero() {
    let data = "\"71720 53977 4\"";
    let deserialized: GTIN = serde_json::from_str(data).unwrap();
    match deserialized {
        GTIN::UpcA(digits) => assert_eq!(digits, [0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]),
        _ => panic!("Deserialized to incorrect type"),
    }
}

#[test]
#[cfg(feature = "serde")]
fn round_trip_serialization() {
    let gtin = GTIN::UpcA([0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]);
    let serialized = serde_json::to_string(&gtin).unwrap();
    let deserialized: GTIN = serde_json::from_str(&serialized).unwrap();
    assert_eq!(gtin, deserialized);
}

#[test]
#[cfg(feature = "serde")]
fn json_serialize_product() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Product {
        name: String,
        gtin: GTIN,
    }
    let product = Product {
        name: "Oreo".to_string(),
        gtin: GTIN::UpcA([0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]),
    };
    let serialized = serde_json::to_string(&product).unwrap();
    assert_eq!(serialized, r#"{"name":"Oreo","gtin":"071720539774"}"#);
}

#[test]
#[cfg(feature = "serde")]
fn json_deserialize_product() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Product {
        name: String,
        gtin: GTIN,
    }
    let json_data = r#"{"name":"Oreo","gtin":"0 71720 53977 4"}"#;
    let deserialized: Product = serde_json::from_str(json_data).unwrap();
    let expected = Product {
        name: "Oreo".to_string(),
        gtin: GTIN::UpcA([0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]),
    };
    assert_eq!(deserialized, expected);
}

#[test]
#[cfg(feature = "serde")]
fn deserialize_invalid_gtin() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Product {
        name: String,
        gtin: GTIN,
    }
    // Invalid GTIN, check digit should be 4, not 5
    let json_data = r#"{"name":"Oreo","gtin":"071720539775"}"#;
    let result: Result<Product, serde_json::Error> = serde_json::from_str(json_data);
    assert!(
        result.is_err(),
        "Expected deserialization to fail with an invalid GTIN"
    );
}

#[test]
#[cfg(feature = "sqlx")]
fn sqlx_postgres_traits_are_available() {
    fn assert_type<T>()
    where
        T: sqlx::Type<sqlx::Postgres>,
    {
    }

    fn assert_encode<T>()
    where
        T: sqlx::Type<sqlx::Postgres>,
        for<'q> T: sqlx::Encode<'q, sqlx::Postgres>,
    {
    }

    fn assert_decode<T>()
    where
        for<'r> T: sqlx::Decode<'r, sqlx::Postgres>,
    {
    }

    assert_type::<GTIN>();
    assert_type::<Option<GTIN>>();
    assert_encode::<GTIN>();
    assert_encode::<Option<GTIN>>();
    assert_decode::<GTIN>();
    assert_decode::<Option<GTIN>>();
}
