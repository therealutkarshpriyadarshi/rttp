//! Password hashing utilities using Argon2
//!
//! Provides secure password hashing and verification using the Argon2id algorithm,
//! which is the recommended password hashing function as of 2023.
//!
//! # Example
//! ```rust,ignore
//! use pttp::security::password::PasswordHasher;
//!
//! let hasher = PasswordHasher::new();
//! let hash = hasher.hash_password("my-secure-password")?;
//! assert!(hasher.verify_password("my-secure-password", &hash)?);
//! ```

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher as _, PasswordVerifier, SaltString},
    Argon2,
};

/// Password hasher using Argon2id
#[derive(Clone)]
pub struct PasswordHasher {
    argon2: Argon2<'static>,
}

impl PasswordHasher {
    /// Create a new password hasher with default parameters
    pub fn new() -> Self {
        Self {
            argon2: Argon2::default(),
        }
    }

    /// Hash a password and return the PHC string
    ///
    /// The returned string contains the algorithm, parameters, salt, and hash
    /// in the PHC (Password Hashing Competition) format.
    pub fn hash_password(&self, password: impl AsRef<[u8]>) -> Result<String, PasswordError> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = self
            .argon2
            .hash_password(password.as_ref(), &salt)
            .map_err(|e| PasswordError::HashingFailed(e.to_string()))?;

        Ok(hash.to_string())
    }

    /// Verify a password against a hash
    ///
    /// Returns `Ok(true)` if the password matches, `Ok(false)` if it doesn't,
    /// and `Err` if the hash is invalid.
    pub fn verify_password(&self, password: impl AsRef<[u8]>, hash: &str) -> Result<bool, PasswordError> {
        let parsed_hash = PasswordHash::new(hash).map_err(|e| PasswordError::InvalidHash(e.to_string()))?;

        match self.argon2.verify_password(password.as_ref(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(PasswordError::VerificationFailed(e.to_string())),
        }
    }

    /// Check if a hash needs to be rehashed (for security upgrades)
    ///
    /// This can be used to upgrade hashes when security parameters change.
    pub fn needs_rehash(&self, hash: &str) -> bool {
        // For simplicity, we don't implement parameter checking here
        // In production, you'd check if the hash parameters match current standards
        false
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Password-related errors
#[derive(Debug, Clone)]
pub enum PasswordError {
    /// Password hashing failed
    HashingFailed(String),
    /// Password hash is invalid
    InvalidHash(String),
    /// Password verification failed
    VerificationFailed(String),
    /// Password is too weak
    WeakPassword(String),
}

impl std::fmt::Display for PasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HashingFailed(msg) => write!(f, "Password hashing failed: {}", msg),
            Self::InvalidHash(msg) => write!(f, "Invalid password hash: {}", msg),
            Self::VerificationFailed(msg) => write!(f, "Password verification failed: {}", msg),
            Self::WeakPassword(msg) => write!(f, "Password is too weak: {}", msg),
        }
    }
}

impl std::error::Error for PasswordError {}

/// Password strength validator
pub struct PasswordValidator {
    min_length: usize,
    require_uppercase: bool,
    require_lowercase: bool,
    require_digit: bool,
    require_special: bool,
}

impl PasswordValidator {
    /// Create a new password validator with default rules
    pub fn new() -> Self {
        Self {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: false,
        }
    }

    /// Set minimum password length
    pub fn min_length(mut self, length: usize) -> Self {
        self.min_length = length;
        self
    }

    /// Require at least one uppercase letter
    pub fn require_uppercase(mut self, required: bool) -> Self {
        self.require_uppercase = required;
        self
    }

    /// Require at least one lowercase letter
    pub fn require_lowercase(mut self, required: bool) -> Self {
        self.require_lowercase = required;
        self
    }

    /// Require at least one digit
    pub fn require_digit(mut self, required: bool) -> Self {
        self.require_digit = required;
        self
    }

    /// Require at least one special character
    pub fn require_special(mut self, required: bool) -> Self {
        self.require_special = required;
        self
    }

    /// Validate a password against the configured rules
    pub fn validate(&self, password: &str) -> Result<(), PasswordError> {
        if password.len() < self.min_length {
            return Err(PasswordError::WeakPassword(format!(
                "Password must be at least {} characters",
                self.min_length
            )));
        }

        if self.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            return Err(PasswordError::WeakPassword("Password must contain at least one uppercase letter".to_string()));
        }

        if self.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            return Err(PasswordError::WeakPassword("Password must contain at least one lowercase letter".to_string()));
        }

        if self.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
            return Err(PasswordError::WeakPassword("Password must contain at least one digit".to_string()));
        }

        if self.require_special && !password.chars().any(|c| !c.is_alphanumeric()) {
            return Err(PasswordError::WeakPassword("Password must contain at least one special character".to_string()));
        }

        Ok(())
    }
}

impl Default for PasswordValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let hasher = PasswordHasher::new();
        let password = "secure-password-123";

        let hash = hasher.hash_password(password).expect("Failed to hash");
        assert!(hasher.verify_password(password, &hash).expect("Failed to verify"));
    }

    #[test]
    fn test_verify_wrong_password() {
        let hasher = PasswordHasher::new();
        let password = "secure-password-123";
        let wrong_password = "wrong-password";

        let hash = hasher.hash_password(password).expect("Failed to hash");
        assert!(!hasher.verify_password(wrong_password, &hash).expect("Failed to verify"));
    }

    #[test]
    fn test_different_hashes() {
        let hasher = PasswordHasher::new();
        let password = "secure-password-123";

        let hash1 = hasher.hash_password(password).expect("Failed to hash");
        let hash2 = hasher.hash_password(password).expect("Failed to hash");

        // Same password should produce different hashes (due to different salts)
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(hasher.verify_password(password, &hash1).expect("Failed to verify"));
        assert!(hasher.verify_password(password, &hash2).expect("Failed to verify"));
    }

    #[test]
    fn test_validator_min_length() {
        let validator = PasswordValidator::new().min_length(10);

        assert!(validator.validate("short").is_err());
        assert!(validator.validate("LongEnough1").is_ok());
    }

    #[test]
    fn test_validator_uppercase() {
        let validator = PasswordValidator::new().require_uppercase(true);

        assert!(validator.validate("nouppercase1").is_err());
        assert!(validator.validate("HasUpper1").is_ok());
    }

    #[test]
    fn test_validator_lowercase() {
        let validator = PasswordValidator::new().require_lowercase(true);

        assert!(validator.validate("NOLOWERCASE1").is_err());
        assert!(validator.validate("HasLower1").is_ok());
    }

    #[test]
    fn test_validator_digit() {
        let validator = PasswordValidator::new().require_digit(true);

        assert!(validator.validate("NoDigits").is_err());
        assert!(validator.validate("HasDigit1").is_ok());
    }

    #[test]
    fn test_validator_special() {
        let validator = PasswordValidator::new().require_special(true);

        assert!(validator.validate("NoSpecial1").is_err());
        assert!(validator.validate("HasSpecial!1").is_ok());
    }

    #[test]
    fn test_validator_all_requirements() {
        let validator = PasswordValidator::new()
            .min_length(12)
            .require_uppercase(true)
            .require_lowercase(true)
            .require_digit(true)
            .require_special(true);

        assert!(validator.validate("weak").is_err());
        assert!(validator.validate("NoSpecial123").is_err());
        assert!(validator.validate("SecurePass123!").is_ok());
    }
}
