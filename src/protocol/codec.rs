use thiserror::Error;

#[derive(Error, Debug)]
pub enum CodecError {
    #[error("Invalid byte length: expected {expected}, got {got}")]
    InvalidLength { expected: usize, got: usize },
}

/// Decode a big-endian IEEE 754 f32
pub fn decode_f32_be(data: &[u8]) -> Result<f32, CodecError> {
    if data.len() < 4 {
        return Err(CodecError::InvalidLength {
            expected: 4,
            got: data.len(),
        });
    }
    Ok(f32::from_be_bytes([data[0], data[1], data[2], data[3]]))
}

/// Encode an f32 as big-endian bytes
pub fn encode_f32_be(value: f32) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

/// Decode a big-endian u32
pub fn decode_u32_be(data: &[u8]) -> Result<u32, CodecError> {
    if data.len() < 4 {
        return Err(CodecError::InvalidLength {
            expected: 4,
            got: data.len(),
        });
    }
    Ok(u32::from_be_bytes([data[0], data[1], data[2], data[3]]))
}

/// Encode a u32 as big-endian bytes
pub fn encode_u32_be(value: u32) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

/// Decode a big-endian u16
pub fn decode_u16_be(data: &[u8]) -> Result<u16, CodecError> {
    if data.len() < 2 {
        return Err(CodecError::InvalidLength {
            expected: 2,
            got: data.len(),
        });
    }
    Ok(u16::from_be_bytes([data[0], data[1]]))
}

/// Encode a u16 as big-endian bytes
pub fn encode_u16_be(value: u16) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

/// Decode a u8 from a byte slice
pub fn decode_u8(data: &[u8]) -> Result<u8, CodecError> {
    data.first().copied().ok_or(CodecError::InvalidLength {
        expected: 1,
        got: 0,
    })
}

/// Encode a u8 as a single byte
pub fn encode_u8(value: u8) -> Vec<u8> {
    vec![value]
}

/// Decode a null-terminated string from BLE data
pub fn decode_string(data: &[u8]) -> String {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).to_string()
}

/// Encode a string as bytes (null-terminated, padded to 8 bytes for View PIN)
pub fn encode_string(s: &str, pad_to: usize) -> Vec<u8> {
    let mut bytes = s.as_bytes().to_vec();
    bytes.resize(pad_to, 0);
    bytes
}

/// Default XOR seed for advertising packet decoding
pub const XOR_SEED: [u8; 10] = [0x5C, 0x6F, 0x2F, 0x41, 0x21, 0x7A, 0x26, 0x45, 0x5C, 0x6F];

/// Compute the effective XOR seed given a View PIN string
pub fn compute_xor_seed(view_pin: &str) -> [u8; 10] {
    let pin_bytes: Vec<u8> = view_pin.bytes().collect();
    let mut seed = XOR_SEED;
    if !pin_bytes.is_empty() {
        for (i, s) in seed.iter_mut().enumerate() {
            *s ^= pin_bytes[i % pin_bytes.len()];
        }
    }
    seed
}

/// XOR-decode/encode advertising data in-place
pub fn xor_advertising(data: &mut [u8], seed: &[u8; 10]) {
    for (i, byte) in data.iter_mut().enumerate() {
        *byte ^= seed[i % 10];
    }
}

/// Decoded B24 advertising packet data
#[derive(Debug, Clone)]
pub struct DecodedAdvert {
    pub data_tag: u16,
    pub status: u8,
    pub units: u8,
    pub value: f32,
    pub pin_valid: bool, // true if verification tags match → View PIN is correct
}

/// Decode a B24 advertising packet from manufacturer data (key 0x04C3).
///
/// B24 advert format:
///   Byte 0:    Format ID (0x01) — plaintext
///   Bytes 1-2: Data Tag (u16 BE) — plaintext (always readable)
///   Byte 3:    Status (u8) — XOR-encoded
///   Byte 4:    Units (u8) — XOR-encoded
///   Bytes 5-8: Data Value (IEEE 754 f32 BE) — XOR-encoded
///   Bytes 9-10: Data Tag verification #1 — XOR-encoded
///   Bytes 11-12: Data Tag verification #2 — XOR-encoded
///
/// Only bytes 3-12 are XOR-encoded with the encoding array (seed ^ View PIN).
/// The encoding array index starts at 0 for byte 3.
pub fn decode_advertising(raw: &[u8], view_pin: &str) -> Option<DecodedAdvert> {
    if raw.len() < 13 {
        return None;
    }

    // Byte 0: Format ID (plaintext, not encoded)
    if raw[0] != 0x01 {
        return None;
    }

    // Bytes 1-2: Data Tag (plaintext — always readable regardless of View PIN)
    let data_tag = u16::from_be_bytes([raw[1], raw[2]]);

    // Bytes 3-12: XOR-encoded portion
    let seed = compute_xor_seed(view_pin);
    let mut encoded = raw[3..13].to_vec();
    xor_advertising(&mut encoded, &seed);

    let status = encoded[0];
    let units = encoded[1];
    let value = f32::from_be_bytes([encoded[2], encoded[3], encoded[4], encoded[5]]);

    // Verify View PIN correctness: decoded tag copies should match plaintext tag
    let verify_tag1 = u16::from_be_bytes([encoded[6], encoded[7]]);
    let verify_tag2 = u16::from_be_bytes([encoded[8], encoded[9]]);
    let pin_valid = verify_tag1 == data_tag && verify_tag2 == data_tag;

    Some(DecodedAdvert {
        data_tag,
        status,
        units,
        value,
        pin_valid,
    })
}
