use rand::{distributions::Alphanumeric, Rng};
use rcgen::{CertificateParams, SanType};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub fn credentials_path() -> PathBuf {
    crate::storage::base_dir().join("credentials.json")
}

pub fn certs_dir() -> PathBuf {
    crate::storage::base_dir().join("certs")
}

pub fn cert_path() -> PathBuf {
    certs_dir().join("arssm.crt.pem")
}

pub fn key_path() -> PathBuf {
    certs_dir().join("arssm.key.pem")
}

pub async fn load_or_create_credentials() -> Result<(Credentials, bool), String> {
    let path = credentials_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(contents) => {
            let creds = serde_json::from_str(&contents)
                .map_err(|err| format!("failed to parse credentials: {err}"))?;
            Ok((creds, false))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let creds = Credentials {
                username: random_token(10),
                password: random_token(20),
            };
            save_credentials(&path, &creds).await?;
            Ok((creds, true))
        }
        Err(err) => Err(format!("failed to read credentials: {err}")),
    }
}

pub async fn save_credentials(path: &Path, creds: &Credentials) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("failed to create credentials dir: {err}"))?;
    }
    let data = serde_json::to_string_pretty(creds)
        .map_err(|err| format!("failed to serialize credentials: {err}"))?;
    tokio::fs::write(path, data)
        .await
        .map_err(|err| format!("failed to write credentials: {err}"))
}

pub async fn ensure_tls_cert(cert_path: &Path, key_path: &Path) -> Result<(), String> {
    if tokio::fs::metadata(cert_path).await.is_ok() && tokio::fs::metadata(key_path).await.is_ok() {
        return Ok(());
    }

    if let Some(parent) = cert_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("failed to create cert dir: {err}"))?;
    }

    let mut params = CertificateParams::new(vec!["localhost".to_string()]);
    params
        .subject_alt_names
        .push(SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)));
    let cert = rcgen::Certificate::from_params(params)
        .map_err(|err| format!("failed to create cert: {err}"))?;

    let cert_pem = cert.serialize_pem().map_err(|err| format!("failed to serialize cert: {err}"))?;
    let key_pem = cert.serialize_private_key_pem();

    tokio::fs::write(cert_path, cert_pem)
        .await
        .map_err(|err| format!("failed to write cert: {err}"))?;
    tokio::fs::write(key_path, key_pem)
        .await
        .map_err(|err| format!("failed to write key: {err}"))?;
    Ok(())
}

fn random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
