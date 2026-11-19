//! Standard base64 (RFC 4648, with padding): the encoding the capnp-JSON codec uses for `Data` with its `base64` rule.
//!
//! Pattern: none (two pure functions).

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The bytes as base64 text.
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0_u32, |a, (i, b)| a | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn sextet(c: u8) -> Option<u32> {
    ALPHABET.iter().position(|a| *a == c).map(|p| p as u32)
}

/// The bytes of base64 text; `None` for anything that is not canonical padded base64.
#[must_use]
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let b = text.as_bytes();
    if b.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(b.len() / 4 * 3);
    for (i, chunk) in b.chunks(4).enumerate() {
        let last = (i + 1) * 4 == b.len();
        let pad = chunk.iter().rev().take_while(|c| **c == b'=').count();
        if pad > 2 || (pad > 0 && !last) {
            return None;
        }
        let mut n = 0_u32;
        for (k, c) in chunk[..4 - pad].iter().enumerate() {
            n |= sextet(*c)? << (18 - 6 * k);
        }
        let bytes = n.to_be_bytes();
        out.extend_from_slice(&bytes[1..4 - pad]);
        // the unused bits of the last sextet must be zero (canonical form)
        if pad > 0 && n & (((1 << (2 * pad)) - 1) << (6 * pad)) != 0 {
            return None;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    #[test]
    fn rfc_4648_vectors() {
        for (plain, text) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(plain.as_bytes()), text);
            assert_eq!(decode(text).as_deref(), Some(plain.as_bytes()));
        }
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(decode(&encode(&all)), Some(all));
    }

    #[test]
    fn what_is_not_canonical_base64_is_refused() {
        for bad in [
            "Zg=", "Zg", "Z===", "Zm9v=", "Zm=v", "Zh==", "Zm9*", "=Zm9", "Zg==Zg==",
        ] {
            assert_eq!(decode(bad), None, "{bad}");
        }
    }
}
