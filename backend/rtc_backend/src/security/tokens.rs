use rand::RngCore;
use sha2::{Digest, Sha256};

pub fn generate_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_token_is_hex_and_64_len() {
        let t = generate_refresh_token();
        assert_eq!(t.len(), 64);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_is_deterministic() {
        let t = "abc";
        assert_eq!(hash_refresh_token(t), hash_refresh_token(t));
        assert_ne!(hash_refresh_token("abc"), hash_refresh_token("abcd"));
    }
}
