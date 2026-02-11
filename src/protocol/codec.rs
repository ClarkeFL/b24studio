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
}

/// Decode a B24 advertising packet from manufacturer data (key 0x04C3).
///
/// B24 advert format (after XOR decoding):
///   Byte 0:   Format ID (0x01)
///   Bytes 1-2: Data Tag (u16 BE)
///   Byte 3:   Status (u8)
///   Byte 4:   Units (u8)
///   Bytes 5-8: Data Value (IEEE 754 f32 BE)
///   Bytes 9-12: Data Tag repeated (verification)
///
/// All bytes are XOR-encoded with the effective seed (base seed ^ View PIN).
pub fn decode_advertising(raw: &[u8], view_pin: &str) -> Option<DecodedAdvert> {
    if raw.len() < 9 {
        return None;
    }
    let mut data = raw.to_vec();
    let seed = compute_xor_seed(view_pin);
    xor_advertising(&mut data, &seed);

    // Validate format ID
    if data[0] != 0x01 {
        return None;
    }

    let data_tag = u16::from_be_bytes([data[1], data[2]]);
    let status = data[3];
    let units = data[4];
    let value = f32::from_be_bytes([data[5], data[6], data[7], data[8]]);

    // Basic validation: reject NaN/Infinity as likely wrong PIN
    if value.is_nan() || value.is_infinite() {
        return None;
    }

    Some(DecodedAdvert {
        data_tag,
        status,
        units,
        value,
    })
}
