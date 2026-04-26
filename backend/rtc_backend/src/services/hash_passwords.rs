use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::errors::AppError;

pub fn hash_passwd(passwd: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(passwd.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::Internal(format!("password hash failed: {e}")))
}

pub fn verify_passwd(passwd: &str, passwd_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(passwd_hash) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(passwd.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::{hash_passwd, verify_passwd};

    #[test]
    fn hash_and_verify_roundtrip() {
        let password = "S3curePassw0rd!";
        let hash = hash_passwd(password).expect("password hash should be generated");

        assert!(verify_passwd(password, &hash));
    }

    #[test]
    fn wrong_password_fails_verification() {
        let hash = hash_passwd("correct-password").expect("password hash should be generated");

        assert!(!verify_passwd("wrong-password", &hash));
    }

    #[test]
    fn hashing_is_salted_and_non_deterministic() {
        let password = "same-input";
        let hash1 = hash_passwd(password).expect("password hash should be generated");
        let hash2 = hash_passwd(password).expect("password hash should be generated");

        assert_ne!(hash1, hash2);
        assert!(verify_passwd(password, &hash1));
        assert!(verify_passwd(password, &hash2));
    }
}
