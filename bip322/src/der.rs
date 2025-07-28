//! DER (Distinguished Encoding Rules) parsing utilities for ECDSA signatures
//!
//! This module provides utilities for parsing ASN.1 DER-encoded ECDSA signatures
//! as used in Bitcoin transactions and BIP-322 message signatures.
//!
//! DER encoding follows a specific structure for ECDSA signatures:
//! ```text
//! 0x30 [total-length] 0x02 [R-length] [R] 0x02 [S-length] [S]
//! ```
//!
//! Where:
//! - `0x30` is the ASN.1 SEQUENCE tag
//! - `[total-length]` is the length of the entire signature content
//! - `0x02` is the ASN.1 INTEGER tag for both R and S values
//! - `[R-length]` and `[S-length]` are the lengths of R and S values respectively
//! - `[R]` and `[S]` are the actual signature components

/// Parse DER length encoding.
///
/// DER uses variable-length encoding for lengths:
/// - Short form: 0-127 (0x00-0x7F) - length in single byte
/// - Long form: 128-255 (0x80-0xFF) - first byte indicates number of length bytes
///
/// # Arguments
///
/// * `bytes` - The bytes starting with the length encoding
///
/// # Returns
///
/// A tuple of (`length_value`, `bytes_consumed`) if parsing succeeds.
pub fn parse_der_length(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.is_empty() {
        return None;
    }

    let first_byte = bytes[0];

    if first_byte & 0x80 == 0 {
        // Short form: length is just the first byte
        Some((usize::from(first_byte), 1))
    } else {
        // Long form: first byte indicates number of length bytes
        let len_bytes = usize::from(first_byte & 0x7F);

        if len_bytes == 0 || len_bytes > 4 || bytes.len() < 1 + len_bytes {
            return None; // Invalid length encoding
        }

        let mut length = 0usize;
        for &byte in bytes.iter().take(len_bytes + 1).skip(1) {
            length = (length << 8) | usize::from(byte);
        }

        // Validate canonical encoding - no leading zeros except for single zero byte
        if len_bytes > 1 && bytes[1] == 0 {
            return None; // Non-canonical: leading zero
        }

        // Validate minimal encoding - could have used short form
        if len_bytes == 1 && length <= 127 {
            return None; // Non-canonical: should use short form
        }
        Some((length, 1 + len_bytes))
    }
}

/// Parse DER-encoded ECDSA signature and extract r, s values.
///
/// This function implements proper ASN.1 DER parsing for ECDSA signatures
/// as used in Bitcoin transactions. It handles the complete DER structure:
///
/// ```text
/// 0x30 [total-length] 0x02 [R-length] [R] 0x02 [S-length] [S]
/// ```
///
/// The function validates:
/// - Correct ASN.1 tags (SEQUENCE 0x30, INTEGER 0x02)
/// - Proper length encoding and consistency
/// - Complete signature structure
///
/// # Arguments
///
/// * `der_bytes` - The DER-encoded signature
///
/// # Returns
///
/// A tuple of (`r_bytes`, `s_bytes`) if parsing succeeds, None otherwise.
pub fn parse_der_ecdsa_signature(der_bytes: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    // DER signature structure:
    // 0x30 [total-length] 0x02 [R-length] [R] 0x02 [S-length] [S]

    if der_bytes.len() < 6 {
        return None; // Too short for minimal DER signature
    }

    let mut pos = 0;

    // Check SEQUENCE tag (0x30)
    if der_bytes[pos] != 0x30 {
        return None;
    }
    pos += 1;

    // Parse total length
    let (total_len, len_bytes) = parse_der_length(&der_bytes[pos..])?;
    pos += len_bytes;

    // Verify total length matches remaining bytes
    if pos + total_len != der_bytes.len() {
        return None;
    }

    // Parse r value
    if pos >= der_bytes.len() || der_bytes[pos] != 0x02 {
        return None; // Missing INTEGER tag for r
    }
    pos += 1;

    let (r_len, len_bytes) = parse_der_length(&der_bytes[pos..])?;
    pos += len_bytes;

    if pos + r_len > der_bytes.len() {
        return None; // r value extends beyond signature
    }

    let r_bytes = der_bytes[pos..pos + r_len].to_vec();
    pos += r_len;

    // Parse s value
    if pos >= der_bytes.len() || der_bytes[pos] != 0x02 {
        return None; // Missing INTEGER tag for s
    }
    pos += 1;

    let (s_len, len_bytes) = parse_der_length(&der_bytes[pos..])?;
    pos += len_bytes;

    if pos + s_len != der_bytes.len() {
        return None; // s value doesn't match remaining bytes
    }

    let s_bytes = der_bytes[pos..pos + s_len].to_vec();

    Some((r_bytes, s_bytes))
}

/// Parse DER signature format (simplified version).
///
/// This is a streamlined version of DER parsing that focuses on extracting
/// the R and S components without extensive validation. Used in signature
/// verification paths where speed is prioritized over comprehensive validation.
///
/// # Arguments
///
/// * `der_bytes` - The DER-encoded signature bytes
///
/// # Returns
///
/// A tuple of (`r_bytes`, `s_bytes`) if parsing succeeds, None otherwise.
pub fn parse_der_signature(der_bytes: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if der_bytes.len() < 6 {
        return None;
    }

    let mut pos = 0;

    // Check DER sequence marker
    if der_bytes[pos] != 0x30 {
        return None;
    }
    pos += 1;

    // Parse total length - we need to validate this matches actual content
    let (total_len, consumed) = parse_der_length(&der_bytes[pos..])?;
    pos += consumed;

    // Parse R value
    if der_bytes[pos] != 0x02 {
        return None;
    }
    pos += 1;

    let (r_len, consumed) = parse_der_length(&der_bytes[pos..])?;
    pos += consumed;

    if pos + r_len > der_bytes.len() {
        return None;
    }

    let r = der_bytes[pos..pos + r_len].to_vec();
    pos += r_len;

    // Parse S value
    if pos >= der_bytes.len() || der_bytes[pos] != 0x02 {
        return None;
    }
    pos += 1;

    let (s_len, consumed) = parse_der_length(&der_bytes[pos..])?;
    pos += consumed;

    if pos + s_len > der_bytes.len() {
        return None;
    }

    let s = der_bytes[pos..pos + s_len].to_vec();
    pos += s_len;

    // Validate that total length matches actual consumed bytes and no trailing data
    let content_start = 1 + consumed; // 1 for sequence tag + consumed for length encoding
    let actual_content_len = pos - content_start;
    let expected_total_bytes = content_start + total_len;

    // Check that content length matches declared length and no trailing data exists
    if actual_content_len != total_len
        || pos != expected_total_bytes
        || expected_total_bytes != der_bytes.len()
    {
        return None; // Length mismatch or trailing data detected
    }

    Some((r, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_der_length_short_form() {
        // Short form: length < 128
        let short_length = vec![0x20]; // Length 32
        let result = parse_der_length(&short_length);
        assert_eq!(result, Some((32, 1)));

        let zero_length = vec![0x00]; // Length 0
        let result = parse_der_length(&zero_length);
        assert_eq!(result, Some((0, 1)));

        let max_short = vec![0x7F]; // Length 127
        let result = parse_der_length(&max_short);
        assert_eq!(result, Some((127, 1)));
    }

    #[test]
    fn test_parse_der_length_long_form() {
        // Long form: length >= 128
        let long_length = vec![0x81, 0xFF]; // Length 255 (1 byte length encoding)
        let result = parse_der_length(&long_length);
        assert_eq!(result, Some((255, 2)));

        let multi_byte = vec![0x82, 0x01, 0x00]; // Length 256 (2 byte length encoding)
        let result = parse_der_length(&multi_byte);
        assert_eq!(result, Some((256, 3)));
    }

    #[test]
    fn test_parse_der_length_invalid() {
        let empty = vec![];
        let result = parse_der_length(&empty);
        assert_eq!(result, None);

        let invalid_long = vec![0x85]; // Claims 5 length bytes but doesn't have them
        let result = parse_der_length(&invalid_long);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_der_ecdsa_signature_trailing_data() {
        // Signature with extra bytes after valid content
        let invalid_der = vec![
            0x30, 0x06, // SEQUENCE, length 6
            0x02, 0x01, 0x01, // INTEGER, length 1, value 0x01 (R)
            0x02, 0x01, 0x02, // INTEGER, length 1, value 0x02 (S)
            0xFF, // Extra byte
        ];
        assert_eq!(parse_der_ecdsa_signature(&invalid_der), None);
    }

    #[test]
    fn test_parse_der_length_non_canonical() {
        // Length 127 encoded in long form (should use short form)
        let non_canonical = vec![0x81, 0x7F];
        // Should fail with canonical validation enabled
        assert_eq!(parse_der_length(&non_canonical), None);
    }

    #[test]
    fn test_parse_der_ecdsa_signature_valid() {
        // Create a minimal valid DER signature for testing
        // 0x30 [len] 0x02 [r-len] [r] 0x02 [s-len] [s]
        let valid_der = vec![
            0x30, 0x06, // SEQUENCE, length 6
            0x02, 0x01, 0x01, // INTEGER, length 1, value 0x01 (R)
            0x02, 0x01, 0x02, // INTEGER, length 1, value 0x02 (S)
        ];

        let result = parse_der_ecdsa_signature(&valid_der);
        assert_eq!(result, Some((vec![0x01], vec![0x02])));
    }

    #[test]
    fn test_parse_der_ecdsa_signature_invalid() {
        // Test various invalid DER structures
        let too_short = vec![0x30, 0x02];
        assert_eq!(parse_der_ecdsa_signature(&too_short), None);

        let wrong_sequence_tag = vec![0x31, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x02];
        assert_eq!(parse_der_ecdsa_signature(&wrong_sequence_tag), None);

        let wrong_integer_tag = vec![0x30, 0x06, 0x03, 0x01, 0x01, 0x02, 0x01, 0x02];
        assert_eq!(parse_der_ecdsa_signature(&wrong_integer_tag), None);

        let length_mismatch = vec![0x30, 0x08, 0x02, 0x01, 0x01, 0x02, 0x01, 0x02]; // Claims length 8 but only has 6 bytes of content
        assert_eq!(parse_der_ecdsa_signature(&length_mismatch), None);
    }

    #[test]
    fn test_parse_der_signature_valid() {
        let valid_der = vec![
            0x30, 0x06, // SEQUENCE, length 6
            0x02, 0x01, 0x01, // INTEGER, length 1, value 0x01 (R)
            0x02, 0x01, 0x02, // INTEGER, length 1, value 0x02 (S)
        ];

        let result = parse_der_signature(&valid_der);
        assert_eq!(result, Some((vec![0x01], vec![0x02])));
    }

    #[test]
    fn test_parse_der_signature_invalid() {
        let too_short = vec![0x30, 0x02];
        assert_eq!(parse_der_signature(&too_short), None);

        let wrong_sequence_tag = vec![0x31, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x02];
        assert_eq!(parse_der_signature(&wrong_sequence_tag), None);
    }

    #[test]
    fn test_parse_der_signature_trailing_data() {
        // Valid DER signature with extra trailing bytes
        let trailing_data = vec![
            0x30, 0x06, // SEQUENCE, length 6
            0x02, 0x01, 0x01, // INTEGER, length 1, value 0x01 (R)
            0x02, 0x01, 0x02, // INTEGER, length 1, value 0x02 (S)
            0xFF, 0xFF, // Extra trailing bytes
        ];
        assert_eq!(parse_der_signature(&trailing_data), None);

        // Valid DER signature with length mismatch (declared length too short)
        let length_mismatch = vec![
            0x30, 0x04, // SEQUENCE, length 4 (but actual content is 6 bytes)
            0x02, 0x01, 0x01, // INTEGER, length 1, value 0x01 (R)
            0x02, 0x01, 0x02, // INTEGER, length 1, value 0x02 (S)
        ];
        assert_eq!(parse_der_signature(&length_mismatch), None);
    }
}
