# gtin

A Rust library for parsing, validating, and working with GTIN (Global Trade Item Number) barcodes.

Supports UPC-A, UPC-E, EAN-8, EAN-13, and GTIN-14 formats.

## Features

- Parse GTINs from strings, handling spaces, hyphens, and other separators
- Automatic format detection by digit count and structure
- Checksum validation using the standard mod-10 algorithm
- Country code lookup via GS1 prefix
- Number system classification (general, ISBN, ISSN, store use, etc.)
- Serde support for JSON serialization/deserialization
- Handles UPC-A codes with stripped leading zeros (11-digit input)

## Usage

```rust
use gtin::GTIN;

// Parse from a string
let barcode: GTIN = "0 71720 53977 4".parse().unwrap();

assert_eq!(barcode.format_name(), "UPC-A");
assert_eq!(barcode.to_string(), "071720539774");
assert_eq!(barcode.country_code(), Some("US"));
```

### Explicit format parsing

8-digit barcodes are ambiguous between UPC-E and EAN-8. By default, codes starting with 0 are parsed as UPC-E and others as EAN-8. Use explicit parsing when you know the format:

```rust
use gtin::GTIN;

let ean8 = GTIN::parse_ean8("52013485").unwrap();
let upce = GTIN::parse_upce("04182634").unwrap();
```

### Serde support

GTINs serialize as digit strings and deserialize with the same flexible parsing as `parse()`:

```rust
use gtin::GTIN;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Product {
    name: String,
    gtin: GTIN,
}

let json = r#"{"name": "Oreo", "gtin": "0 71720 53977 4"}"#;
let product: Product = serde_json::from_str(json).unwrap();
```

### Error handling

Parsing returns `GtinError` with specific variants:

```rust
use gtin::{GTIN, GtinError};

assert_eq!(
    GTIN::try_from("12345"),
    Err(GtinError::InvalidLength(5))
);
assert_eq!(
    GTIN::try_from("071720539775"),
    Err(GtinError::InvalidChecksum)
);
```

## Supported formats

| Format  | Digits | Example        |
|---------|--------|----------------|
| UPC-E   | 8      | `04182634`     |
| EAN-8   | 8      | `52013485`     |
| UPC-A   | 12     | `071720539774` |
| EAN-13  | 13     | `8595701530526`|
| GTIN-14 | 14     | `00012345678905`|
