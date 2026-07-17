# gtin

A Rust library for parsing, validating, and working with GTIN (Global Trade Item Number) barcodes.

Supports UPC-A, UPC-E, EAN-8, EAN-13, and GTIN-14 formats.

## Features

- Parse GTINs from strings, handling spaces, hyphens, and other separators
- Automatic format detection by digit count and structure
- Checksum validation using the standard mod-10 algorithm
- Optional random GTIN generation with valid checksums
- Country code lookup via GS1 prefix
- Number system classification (general, ISBN, ISSN, store use, etc.)
- Optional serde support for JSON serialization/deserialization
- Optional SQLx support for PostgreSQL text columns
- Handles UPC-A codes with stripped leading zeros (11-digit input)
- Normalizes short-format codes stored in expanded form back to 8 digits
  (UPC-E from its UPC-A expansion, EAN-8 from zero-padded
  12/13/14-digit strings)

## Cargo features

This crate has no default features. Enable optional functionality as needed:

```toml
[dependencies]
gtin = { version = "0.5", features = ["random", "serde", "sqlx"] }
```

| Feature  | Enables                                               |
|----------|-------------------------------------------------------|
| `random` | Random GTIN generation via `rand`                     |
| `serde`  | `Serialize`/`Deserialize` impls for `GTIN`            |
| `sqlx`   | `Type`/`Encode`/`Decode` impls for SQLx + PostgreSQL  |

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

### Short-format normalization

Some systems store short-format barcodes in an expanded or zero-padded form.
Parsing normalizes these back to their canonical 8-digit form: a UPC-A
matching a GS1 zero-suppression pattern is returned as UPC-E, and a code
whose digits are all zero except the trailing eight is returned as EAN-8.
When a code qualifies for both, the EAN-8 interpretation wins, since GS1
reserves the leading-zeros space for GTIN-8.

```rust
use gtin::GTIN;

// A UPC-E that was stored as its 12-digit UPC-A expansion.
let upce: GTIN = "041800000265".parse().unwrap();
assert_eq!(upce.format_name(), "UPC-E");
assert_eq!(upce.to_string(), "04182635");

// An EAN-8 that was stored zero-padded to 13 digits.
let ean8: GTIN = "0000052013485".parse().unwrap();
assert_eq!(ean8.format_name(), "EAN-8");
assert_eq!(ean8.to_string(), "52013485");
```

The conversions are also available directly on constructed values as
`as_upce` and `as_ean8`, mirroring `as_upca` and `as_ean13`. `as_upce` is
the exact inverse of UPC-E expansion, so it only succeeds for codes matching
a zero-suppression pattern. `as_ean8` succeeds only when every digit before
the trailing eight is zero and the first EAN-8 digit is non-zero; genuine
EAN-8 codes are assigned from their own GS1 namespace, so no other
conversion from a longer code exists.

### Random generation

Requires the `random` feature.

Generate a random GTIN with a valid checksum, optionally choosing the format:

```rust
use gtin::{GTIN, GtinType};

let any = GTIN::random();
let upca = GTIN::random_of_type(GtinType::UpcA);
let ean13 = GTIN::random_of_type(GtinType::Ean13);

assert!(GTIN::try_from(any.to_string().as_str()).is_ok());
assert_eq!(upca.format_name(), "UPC-A");
assert_eq!(ean13.format_name(), "EAN-13");
```

### Serde support

Requires the `serde` feature.

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

### SQLx support

Requires the `sqlx` feature.

GTINs encode to PostgreSQL as canonical digit strings and decode from PostgreSQL text-like columns:

```rust
use gtin::GTIN;

let gtin: GTIN = "0 71720 53977 4".parse().unwrap();

sqlx::query("insert into products (gtin) values ($1)")
    .bind(gtin)
    .execute(&pool)
    .await?;

let selected: GTIN = sqlx::query_scalar("select gtin from products limit 1")
    .fetch_one(&pool)
    .await?;
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
