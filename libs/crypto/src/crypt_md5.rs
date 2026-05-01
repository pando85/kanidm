use md5::{Digest, Md5};
use std::cmp::min;

/// Maximum salt length.
const MD5_MAGIC: &str = "$1$";
const MD5_TRANSPOSE: &[u8] = b"\x0c\x06\x00\x0d\x07\x01\x0e\x08\x02\x0f\x09\x03\x05\x0a\x04\x0b";

const CRYPT_HASH64: &[u8] = b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

pub fn md5_sha2_hash64_encode(bs: &[u8]) -> String {
    let ngroups = bs.len().div_ceil(3);
    let mut out = String::with_capacity(ngroups * 4);
    for g in 0..ngroups {
        let mut g_idx = g * 3;
        let mut enc = 0u32;
        #[allow(clippy::explicit_counter_loop)]
        for _ in 0..3 {
            let b = (if g_idx < bs.len() { bs[g_idx] } else { 0 }) as u32;
            enc >>= 8;
            enc |= b << 16;
            g_idx += 1;
        }
        for _ in 0..4 {
            out.push(char::from_u32(CRYPT_HASH64[(enc & 0x3F) as usize] as u32).unwrap_or('!'));
            enc >>= 6;
        }
    }
    match bs.len() % 3 {
        1 => {
            out.pop();
            out.pop();
        }
        2 => {
            out.pop();
        }
        _ => (),
    }
    out
}

pub fn do_md5_crypt(pass: &[u8], salt: &[u8]) -> Vec<u8> {
    let mut dgst_b = Md5::new();
    dgst_b.update(pass);
    dgst_b.update(salt);
    dgst_b.update(pass);
    let mut hash_b = dgst_b.finalize();

    let mut dgst_a = Md5::new();
    dgst_a.update(pass);
    dgst_a.update(MD5_MAGIC.as_bytes());
    dgst_a.update(salt);

    let mut plen = pass.len();
    while plen > 0 {
        dgst_a.update(&hash_b[..min(plen, 16)]);
        if plen < 16 {
            break;
        }
        plen -= 16;
    }

    plen = pass.len();
    while plen > 0 {
        if plen & 1 == 0 {
            dgst_a.update(&pass[..1])
        } else {
            dgst_a.update([0u8])
        }
        plen >>= 1;
    }

    let mut hash_a = dgst_a.finalize();

    for r in 0..1000 {
        let mut dgst_a = Md5::new();
        if r % 2 == 1 {
            dgst_a.update(pass);
        } else {
            dgst_a.update(hash_a);
        }
        if r % 3 > 0 {
            dgst_a.update(salt);
        }
        if r % 7 > 0 {
            dgst_a.update(pass);
        }
        if r % 2 == 0 {
            dgst_a.update(pass);
        } else {
            dgst_a.update(hash_a);
        }
        hash_a = dgst_a.finalize();
    }

    for (i, &ti) in MD5_TRANSPOSE.iter().enumerate() {
        hash_b[i] = hash_a[ti as usize];
    }

    md5_sha2_hash64_encode(&hash_b).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash64_encode_empty() {
        let result = md5_sha2_hash64_encode(&[]);
        assert_eq!(result, "");
    }

    #[test]
    fn test_hash64_encode_single_byte() {
        // Single byte: 0x00 -> ".." (2 chars for 1 byte mod 3 == 1)
        let result = md5_sha2_hash64_encode(&[0x00]);
        assert_eq!(result, "..");
    }

    #[test]
    fn test_hash64_encode_two_bytes() {
        // Two bytes: mod 3 == 2, should produce 3 chars
        let result = md5_sha2_hash64_encode(&[0x00, 0x00]);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_hash64_encode_three_bytes() {
        // Three bytes: exact group, should produce 4 chars
        let result = md5_sha2_hash64_encode(&[0x00, 0x00, 0x00]);
        assert_eq!(result, "....");
    }

    #[test]
    fn test_hash64_encode_sixteen_bytes() {
        // Standard MD5 hash length (16 bytes): 16 % 3 == 1, so 22 chars
        let input = [0u8; 16];
        let result = md5_sha2_hash64_encode(&input);
        assert_eq!(result.len(), 22);
    }

    #[test]
    fn test_hash64_encode_known_value() {
        // Test with a known pattern to verify encoding correctness
        // 16 bytes of 0xff
        let input = [0xFFu8; 16];
        let result = md5_sha2_hash64_encode(&input);
        assert_eq!(result.len(), 22);
        // All characters should be from the CRYPT_HASH64 alphabet
        for c in result.chars() {
            assert!(
                CRYPT_HASH64.contains(&(c as u8)),
                "Invalid character '{}' in output",
                c
            );
        }
    }

    #[test]
    fn test_hash64_encode_length_mod_3_is_1() {
        // 4 bytes: 4 % 3 == 1, should produce 6 chars (8 - 2)
        let result = md5_sha2_hash64_encode(&[0x00, 0x00, 0x00, 0x00]);
        assert_eq!(result.len(), 6);
    }

    #[test]
    fn test_hash64_encode_length_mod_3_is_2() {
        // 5 bytes: 5 % 3 == 2, should produce 7 chars (8 - 1)
        let result = md5_sha2_hash64_encode(&[0x00, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(result.len(), 7);
    }

    #[test]
    fn test_hash64_encode_all_chars_in_alphabet() {
        // Test with varied input to ensure output only uses valid characters
        let input: Vec<u8> = (0..32).collect();
        let result = md5_sha2_hash64_encode(&input);
        for c in result.chars() {
            assert!(
                CRYPT_HASH64.contains(&(c as u8)),
                "Invalid character '{}' in output",
                c
            );
        }
    }

    // Test vectors generated with: openssl passwd -1 -salt SALT PASSWORD
    #[test]
    fn test_md5_crypt_password_salt_12345678() {
        // openssl passwd -1 -salt "12345678" "password"
        // Output: $1$12345678$o2n/JiO/h5VviOInWJ4OQ/
        let result = do_md5_crypt(b"password", b"12345678");
        assert_eq!(result, b"o2n/JiO/h5VviOInWJ4OQ/");
    }

    #[test]
    fn test_md5_crypt_hello_world() {
        // openssl passwd -1 -salt "world" "hello"
        // Output: $1$world$9570e6rgzRsa0dGF.W93f.
        let result = do_md5_crypt(b"hello", b"world");
        assert_eq!(result, b"9570e6rgzRsa0dGF.W93f.");
    }

    #[test]
    fn test_md5_crypt_empty_password() {
        // openssl passwd -1 -salt "12345678" ""
        // Output: $1$12345678$xek.CpjQUVgdf/P2N9KQf/
        let result = do_md5_crypt(b"", b"12345678");
        assert_eq!(result, b"xek.CpjQUVgdf/P2N9KQf/");
    }

    #[test]
    fn test_md5_crypt_deterministic() {
        // Ensure the function is deterministic
        let result1 = do_md5_crypt(b"test_password", b"testsalt");
        let result2 = do_md5_crypt(b"test_password", b"testsalt");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_md5_crypt_different_salts() {
        // Different salts should produce different hashes
        let result1 = do_md5_crypt(b"password", b"salt1");
        let result2 = do_md5_crypt(b"password", b"salt2");
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_md5_crypt_different_passwords() {
        // Different passwords should produce different hashes
        let result1 = do_md5_crypt(b"password1", b"salt");
        let result2 = do_md5_crypt(b"password2", b"salt");
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_md5_crypt_long_password() {
        // Test password longer than 16 bytes (tests the while loop in do_md5_crypt)
        let long_password = b"this_is_a_very_long_password_that_exceeds_sixteen_bytes";
        let result = do_md5_crypt(long_password, b"longsalt");
        assert_eq!(result.len(), 22);
        // Verify all characters are valid
        for &b in &result {
            assert!(CRYPT_HASH64.contains(&b), "Invalid byte {} in output", b);
        }
    }

    #[test]
    fn test_md5_crypt_special_characters() {
        // Test with special characters in password
        let result = do_md5_crypt(b"p@ss!w0rd#$%^&*()", b"special");
        assert_eq!(result.len(), 22);
    }

    #[test]
    fn test_md5_crypt_binary_data() {
        // Test with binary data in password
        let binary_password: &[u8] = &[0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
        let result = do_md5_crypt(binary_password, b"binary");
        assert_eq!(result.len(), 22);
    }

    #[test]
    fn test_md5_crypt_output_length() {
        // All outputs should be 22 characters (standard crypt-md5 hash length)
        for (pass, salt) in &[
            (b"a" as &[u8], b"s" as &[u8]),
            (b"short", b"sal"),
            (b"medium_length_pw", b"medium_salt"),
            (b"x".repeat(100).as_slice(), b"long_salt_value"),
        ] {
            let result = do_md5_crypt(pass, salt);
            assert_eq!(
                result.len(),
                22,
                "Output length mismatch for pass={:?}, salt={:?}",
                pass,
                salt
            );
        }
    }
}
