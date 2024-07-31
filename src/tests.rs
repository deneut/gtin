use crate::{NumberSystem, GTIN};

#[test]
fn determine_number_system() {
    let cases = vec![
        ("8595701 530526", NumberSystem::General),  // EAN-13
        ("8595701 542376", NumberSystem::General),  // EAN-13
        ("8 595682 148871", NumberSystem::General), // EAN-13
        ("0 71720 53977 4", NumberSystem::General), // UPC-A
        ("0 41420 06785 3", NumberSystem::General), // UPC-A
        // ("5201 3485", NumberSystem::General),          // EAN-8 TODO: Implement EAN-8
        ("9783161484100", NumberSystem::Isbn),         // ISBN
        ("9772434561006", NumberSystem::Issn),         // ISSN
        ("02 45678 1 0543 9", NumberSystem::StoreUse), // Store Use, variable
    ];

    cases.into_iter().for_each(|(gtin, number_system)| {
        let gtin = crate::GTIN::try_from(gtin).unwrap();
        assert_eq!(
            gtin.number_system(),
            number_system,
            "Failed to match GTIN: {}",
            gtin
        );
    });
}

#[test]
fn determine_country_code() {
    let cases = vec![
        ("8595701 530526", Some("CZ")),  // EAN-13
        ("8595701 542376", Some("CZ")),  // EAN-13
        ("8 595682 148871", Some("CZ")), // EAN-13
        ("0 71720 53977 4", Some("US")), // UPC-A
        ("0 41420 06785 3", Some("US")), // UPC-A
        ("02 45678 1 0543 9", None),     // Store Use, variable
    ];

    cases.into_iter().for_each(|(gtin, country_code)| {
        let gtin = crate::GTIN::try_from(gtin).unwrap();
        assert_eq!(
            gtin.country_code(),
            country_code,
            "Failed to match GTIN: {}",
            gtin
        );
    });
}

// serde tests

#[test]
fn serialize_upca() {
    let gtin = GTIN::UpcA([0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]);
    let serialized = serde_json::to_string(&gtin).unwrap();
    assert_eq!(serialized, "\"071720539774\"");
}

#[test]
fn deserialize_upca_with_spaces() {
    let data = "\"0 71720 53977 4\"";
    let deserialized: GTIN = serde_json::from_str(data).unwrap();
    match deserialized {
        GTIN::UpcA(digits) => assert_eq!(digits, [0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]),
        _ => panic!("Deserialized to incorrect type"),
    }
}

#[test]
fn deserialize_upca_with_spaces_and_missing_initial_zero() {
    let data = "\"71720 53977 4\"";
    let deserialized: GTIN = serde_json::from_str(data).unwrap();
    match deserialized {
        GTIN::UpcA(digits) => assert_eq!(digits, [0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]),
        _ => panic!("Deserialized to incorrect type"),
    }
}

#[test]
fn round_trip_serialization() {
    let gtin = GTIN::UpcA([0, 7, 1, 7, 2, 0, 5, 3, 9, 7, 7, 4]);
    let serialized = serde_json::to_string(&gtin).unwrap();
    let deserialized: GTIN = serde_json::from_str(&serialized).unwrap();
    assert_eq!(gtin, deserialized);
}

#[test]
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
fn deserialize_invalid_gtin() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Product {
        name: String,
        gtin: GTIN,
    }
    let json_data = r#"{"name":"Oreo","gtin":"071720539775"}"#; // Invalid GTIN, check digit should be 4, not 5
    let result: Result<Product, serde_json::Error> = serde_json::from_str(json_data);
    assert!(
        result.is_err(),
        "Expected deserialization to fail with an invalid GTIN"
    );
}
