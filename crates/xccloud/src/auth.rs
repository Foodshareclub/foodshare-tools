//! App Store Connect credentials and the ES256 token they sign.
//!
//! The App Store Connect API authenticates with a JSON Web Token signed
//! with the private key from App Store Connect, in the ES256 form: the
//! signature is the raw `r || s` pair, not the DER encoding.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::pkcs8::DecodePrivateKey;
use serde::{Deserialize, Serialize};

/// The audience an App Store Connect token is for.
pub const AUDIENCE: &str = "appstoreconnect-v1";

/// How long a token is valid for.
pub const TOKEN_LIFETIME_SECS: i64 = 20 * 60;

/// The issuer this repository falls back to.
pub const DEFAULT_ISSUER_ID: &str = "69a6de8f-c4a2-47e3-e053-5b8c7c11a4d1";

/// The key identifiers this repository has used.
pub const KNOWN_KEY_CANDIDATES: &[&str] = &["3B28BL42D3", "4N7PTU3PVD"];

/// The credentials the App Store Connect API needs.
#[derive(Debug, Clone)]
pub struct Credentials {
    /// The key identifier.
    pub key_id: String,
    /// The issuer identifier.
    pub issuer_id: String,
    /// The private key, in PEM form.
    pub private_key: String,
}

/// The JSON Web Token header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHeader {
    /// The signing algorithm.
    pub alg: String,
    /// The key identifier.
    pub kid: String,
    /// The token type.
    pub typ: String,
}

/// The JSON Web Token claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// The issuer.
    pub iss: String,
    /// When the token expires.
    pub exp: i64,
    /// The audience.
    pub aud: String,
}

/// Where the credentials are looked for.
#[derive(Debug, Clone, Default)]
pub struct Locations {
    /// The directories a `.p8` file may live in.
    pub search_dirs: Vec<PathBuf>,
}

/// Find the credentials.
///
/// The environment wins. Otherwise the known key identifiers are tried
/// against the search directories, and finally any `.p8` file in the
/// first search directory is used.
#[must_use]
pub fn resolve(locations: &Locations) -> Option<Credentials> {
    let mut key_id = env_first(&["APP_STORE_CONNECT_KEY_ID", "ASC_KEY_ID"]);
    let issuer_id = env_first(&["APP_STORE_CONNECT_ISSUER_ID", "ASC_ISSUER_ID"])
        .unwrap_or_else(|| DEFAULT_ISSUER_ID.to_string());
    let mut private_key = env_first(&["APP_STORE_CONNECT_PRIVATE_KEY", "ASC_PRIVATE_KEY"]);

    if private_key.is_none() {
        let candidates: Vec<String> = match &key_id {
            Some(id) => vec![id.clone()],
            None => KNOWN_KEY_CANDIDATES
                .iter()
                .map(|k| (*k).to_string())
                .collect(),
        };
        for candidate in &candidates {
            for dir in &locations.search_dirs {
                for name in [format!("AuthKey_{candidate}.p8"), format!("{candidate}.p8")] {
                    if let Some(contents) = read_key(&dir.join(name)) {
                        private_key = Some(contents);
                        key_id = Some(candidate.clone());
                        break;
                    }
                }
                if private_key.is_some() {
                    break;
                }
            }
            if private_key.is_some() {
                break;
            }
        }

        // Any `.p8` file will do, when the identifier is in the name.
        if private_key.is_none() {
            if let Some(dir) = locations.search_dirs.first() {
                for entry in std::fs::read_dir(dir).ok().into_iter().flatten() {
                    let Ok(entry) = entry else { continue };
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("p8") {
                        continue;
                    }
                    let Some(contents) = read_key(&path) else {
                        continue;
                    };
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default();
                    if let Some(id) = key_id_from_file_name(name) {
                        key_id = Some(id);
                    }
                    private_key = Some(contents);
                    break;
                }
            }
        }
    }

    match (key_id, private_key) {
        (Some(key_id), Some(private_key)) => Some(Credentials {
            key_id,
            issuer_id,
            private_key,
        }),
        _ => None,
    }
}

/// The key identifier in a `.p8` file name.
///
/// Both `AuthKey_ABC123.p8` and `ABC123.p8` are accepted.
#[must_use]
pub fn key_id_from_file_name(name: &str) -> Option<String> {
    let stem = name.strip_suffix(".p8")?;
    if let Some(rest) = stem.strip_prefix("AuthKey_") {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Some(rest.to_string());
        }
    }
    if !stem.is_empty() && stem.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Some(stem.to_string());
    }
    None
}

/// Sign a token for the App Store Connect API.
pub fn create_token(credentials: &Credentials, now: i64) -> Result<String, String> {
    let header = TokenHeader {
        alg: "ES256".to_string(),
        kid: credentials.key_id.clone(),
        typ: "JWT".to_string(),
    };
    let claims = TokenClaims {
        iss: credentials.issuer_id.clone(),
        exp: now + TOKEN_LIFETIME_SECS,
        aud: AUDIENCE.to_string(),
    };

    let encoded_header = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&header).map_err(|e| format!("could not encode the header: {e}"))?,
    );
    let encoded_claims = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&claims).map_err(|e| format!("could not encode the claims: {e}"))?,
    );
    let signing_input = format!("{encoded_header}.{encoded_claims}");

    let signature = sign(&credentials.private_key, signing_input.as_bytes())?;
    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature)
    ))
}

/// Sign a message with a PEM private key.
///
/// The signature is the raw `r || s` pair, which is what ES256 in a JSON
/// Web Token carries.
fn sign(private_key_pem: &str, message: &[u8]) -> Result<Vec<u8>, String> {
    let der = pem_to_der(private_key_pem)?;
    let key = SigningKey::from_pkcs8_der(&der)
        .map_err(|e| format!("could not read the private key: {e}"))?;
    let signature: Signature = key.sign(message);
    Ok(signature.to_bytes().to_vec())
}

/// The DER bytes of a PEM private key.
///
/// The PEM body is base64, with the header, the footer and the line
/// breaks removed.
fn pem_to_der(pem: &str) -> Result<Vec<u8>, String> {
    let body: String = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");
    let body: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD
        .decode(body)
        .map_err(|e| format!("the private key is not valid base64: {e}"))
}

/// The first environment variable that is set.
fn env_first(names: &[&str]) -> Option<String> {
    names
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.is_empty())
}

/// Read a key file.
fn read_key(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// The directories the credentials are looked for in.
#[must_use]
pub fn default_locations(app_dir: &Path) -> Locations {
    let mut search_dirs = vec![app_dir.join("keys")];
    if let Some(home) = std::env::var_os("HOME") {
        search_dirs.push(
            PathBuf::from(home)
                .join(".appstoreconnect")
                .join("private_keys"),
        );
    }
    search_dirs.push(app_dir.to_path_buf());
    Locations { search_dirs }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::VerifyingKey;
    use p256::ecdsa::signature::Verifier;
    use p256::pkcs8::{EncodePrivateKey, LineEnding};

    /// A P-256 key, for the test.
    ///
    /// The secret is a fixed value: the test needs a key that signs, not
    /// a key that is unpredictable.
    fn test_key() -> (String, VerifyingKey) {
        let secret = p256::SecretKey::from_slice(&[7u8; 32]).expect("secret");
        let key = SigningKey::from(secret);
        let public = *key.verifying_key();
        let pem = key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("encode")
            .to_string();
        (pem, public)
    }

    #[test]
    fn test_key_id_from_file_name() {
        assert_eq!(
            key_id_from_file_name("AuthKey_3B28BL42D3.p8").as_deref(),
            Some("3B28BL42D3")
        );
        assert_eq!(
            key_id_from_file_name("4N7PTU3PVD.p8").as_deref(),
            Some("4N7PTU3PVD")
        );
        assert_eq!(key_id_from_file_name("notes.txt"), None);
        assert_eq!(key_id_from_file_name("AuthKey_.p8"), None);
    }

    #[test]
    fn test_pem_to_der_strips_the_armour() {
        let (pem, _) = test_key();
        let der = pem_to_der(&pem).expect("der");
        assert!(!der.is_empty());
    }

    #[test]
    fn test_pem_to_der_rejects_junk() {
        assert!(pem_to_der("not a key").is_err());
    }

    #[test]
    fn test_create_token_is_three_parts() {
        let (pem, _) = test_key();
        let credentials = Credentials {
            key_id: "ABC123".to_string(),
            issuer_id: "issuer".to_string(),
            private_key: pem,
        };
        let token = create_token(&credentials, 1_700_000_000).expect("token");
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);

        let header: TokenHeader = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts[0])
                .expect("header"),
        )
        .expect("header json");
        assert_eq!(header.alg, "ES256");
        assert_eq!(header.kid, "ABC123");
        assert_eq!(header.typ, "JWT");

        let claims: TokenClaims = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts[1])
                .expect("claims"),
        )
        .expect("claims json");
        assert_eq!(claims.iss, "issuer");
        assert_eq!(claims.aud, AUDIENCE);
        assert_eq!(claims.exp, 1_700_000_000 + TOKEN_LIFETIME_SECS);
    }

    #[test]
    fn test_the_signature_verifies() {
        let (pem, public) = test_key();
        let credentials = Credentials {
            key_id: "ABC123".to_string(),
            issuer_id: "issuer".to_string(),
            private_key: pem,
        };
        let token = create_token(&credentials, 1_700_000_000).expect("token");
        let parts: Vec<&str> = token.split('.').collect();

        let signature_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[2])
            .expect("signature");
        // ES256 in a JSON Web Token carries the raw pair, not DER.
        assert_eq!(signature_bytes.len(), 64);

        let signature = Signature::from_slice(&signature_bytes).expect("signature");
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        public
            .verify(signing_input.as_bytes(), &signature)
            .expect("the signature verifies");
    }

    #[test]
    fn test_resolve_finds_a_key_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let keys = dir.path().join("keys");
        std::fs::create_dir_all(&keys).expect("create");
        std::fs::write(keys.join("AuthKey_ABC123.p8"), "key-material").expect("write");

        let locations = Locations {
            search_dirs: vec![keys],
        };
        let credentials = resolve(&locations).expect("credentials");
        assert_eq!(credentials.key_id, "ABC123");
        assert_eq!(credentials.private_key, "key-material");
        assert_eq!(credentials.issuer_id, DEFAULT_ISSUER_ID);
    }

    #[test]
    fn test_resolve_without_a_key_is_none() {
        let dir = tempfile::tempdir().expect("temp dir");
        let locations = Locations {
            search_dirs: vec![dir.path().to_path_buf()],
        };
        assert!(resolve(&locations).is_none());
    }
}
