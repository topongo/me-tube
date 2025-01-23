use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use rand::{rngs::OsRng, RngCore};
use base64::{Engine, prelude::BASE64_STANDARD};
use argon2::{Argon2, PasswordHasher, PasswordVerifier, PasswordHash, password_hash::SaltString};

use crate::config::CONFIG;

fn secure_rnd_string() -> String {
    let mut rng = OsRng;
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    BASE64_STANDARD.encode(bytes)
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ExpiringToken {
    token: String,
    expires: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct User {
    pub username: String,
    password_hash: String,
    access_token: Option<ExpiringToken>,
    refresh_token: Option<ExpiringToken>,
}

impl User {
    fn password_hash(password: String) -> String {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string()
    }

    pub(crate) fn new(username: String, password: String) -> Self {
        Self {
            username,
            password_hash: Self::password_hash(password),
            access_token: None,
            refresh_token: None,
        }
    }

    pub(crate) fn verify_password(&self, password: String) -> bool {
        let argon2 = Argon2::default();
        let hash = PasswordHash::new(&self.password_hash).unwrap();
        argon2.verify_password(password.as_bytes(), &hash).is_ok()
    }

    pub(crate) fn check_access(&self, access_token: &str) -> bool {
        self.access_token.as_ref().map_or(false, |t| t.expires > Utc::now() && t.token == access_token)
    }

    pub(crate) fn check_refresh(&self, refresh_token: &str) -> bool {
        log::debug!("token will expire in {:?}", self.refresh_token.as_ref().map(|t| t.expires - Utc::now()));
        self.refresh_token.as_ref().map_or(false, |t| t.expires > Utc::now() && t.token == refresh_token)
    }

    pub(crate) fn generate_expiring_token(duration: TimeDelta) -> ExpiringToken {
        ExpiringToken {
            token: secure_rnd_string(),
            expires: Utc::now() + duration,
        }
    }

    pub(crate) fn generate_access(&mut self) -> String {
        let access = Self::generate_expiring_token(CONFIG.access_token_duration);
        let r = access.token.clone();
        self.access_token = Some(access);
        r
    }

    pub(crate) fn generate_refresh(&mut self) -> String {
        let refresh = Self::generate_expiring_token(CONFIG.access_token_duration);
        let r = refresh.token.clone();
        self.refresh_token = Some(refresh);
        r
    }

    pub(crate) fn generate_access_and_refresh(&mut self) -> (String, String) {
        (self.generate_access(), self.generate_refresh())
    }
}


