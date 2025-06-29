use std::fs::exists;

use anyhow::Result;
use rcgen::{
  BasicConstraints, CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, Ia5String, IsCa, KeyPair,
  KeyUsagePurpose, SanType,
};
use time::OffsetDateTime;

/// Generates a self-signed CA certificate and its private key.
/// Returns the PEM encoded certificate and private key.
pub fn generate_ca_cert() -> Result<(String, String)> {
  let mut params = CertificateParams::default();
  params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
  params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
  let mut distinguished_name = DistinguishedName::new();
  distinguished_name.push(rcgen::DnType::OrganizationName, "MxLite CA");
  distinguished_name.push(rcgen::DnType::CommonName, "MxLite Root CA");
  params.distinguished_name = distinguished_name;
  // Set a reasonable validity period
  params.not_before = OffsetDateTime::now_utc();
  params.not_after = OffsetDateTime::now_utc() + time::Duration::days(30);

  let key_pair = KeyPair::generate()?;
  let cert = params.self_signed(&key_pair)?;

  let cert_pem = cert.pem();
  let key_pem = key_pair.serialize_pem();

  Ok((cert_pem, key_pem))
}

/// Generates a TLS server certificate signed by the provided CA.
/// Takes the CA certificate PEM, CA private key PEM, and subject alternative names.
/// Returns the PEM encoded server certificate and its private key.
pub fn generate_signed_cert(
  ca_cert_pem: &str, ca_key_pem: &str, subject_alt_names: Vec<String>,
) -> Result<(String, String)> {
  let ca_params = CertificateParams::from_ca_cert_pem(ca_cert_pem)?;
  let mut params = ca_params.clone();

  let ca_key_pair = KeyPair::from_pem(ca_key_pem)?;

  params.subject_alt_names = subject_alt_names
    .into_iter()
    .filter_map(|name| Ia5String::try_from(name).map(SanType::DnsName).ok())
    .collect();
  params.is_ca = IsCa::NoCa;
  params.key_usages = vec![KeyUsagePurpose::DigitalSignature, KeyUsagePurpose::KeyEncipherment];
  params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

  let mut distinguished_name = DistinguishedName::new();
  distinguished_name.push(rcgen::DnType::CommonName, "MxLite Controlplane");
  params.distinguished_name = distinguished_name;

  params.not_before = OffsetDateTime::now_utc();
  params.not_after = OffsetDateTime::now_utc() + time::Duration::days(7);

  let key_pair = KeyPair::generate()?;
  let cert = params.signed_by(&key_pair, &ca_params, &ca_key_pair)?;

  let cert_pem = cert.pem();
  let key_pem = key_pair.serialize_pem();

  Ok((cert_pem, key_pem))
}

/// Reads a certificate and private key from the specified file paths.
///
/// If the files do not exist, it generates a self-signed certificate using the provided CA certificate and key paths.
/// If the CA certificate and key paths are not provided, it returns an error.
pub fn get_cert_from_file(
  cert_path: Option<String>, key_path: Option<String>, ca_cert_path: Option<String>, ca_key_path: Option<String>,
  allow_self_signed: bool,
) -> Result<(String, String)> {
  if let (Some(cert_path), Some(key_path)) = (&cert_path, &key_path) {
    let cert_existed = exists(cert_path)?;
    let key_existed = exists(key_path)?;
    if cert_existed ^ key_existed {
      Err(anyhow::anyhow!(
        "Neither certificate or private key is missing: {}, {}. Keep both existed or both non-existed",
        cert_path,
        key_path
      ))
    } else if cert_existed && key_existed {
      let cert = std::fs::read_to_string(cert_path)?;
      let key = std::fs::read_to_string(key_path)?;
      Ok((cert, key))
    } else if allow_self_signed {
      let (ca_cert, ca_key) = if let (Some(ca_cert_path), Some(ca_key_path)) = (&ca_cert_path, &ca_key_path) {
        let ca_cert_existed = exists(ca_cert_path)?;
        let ca_key_existed = exists(ca_key_path)?;
        if ca_cert_existed ^ ca_key_existed {
          return Err(anyhow::anyhow!(
            "Neither CA certificate or CA private key is missing: {}, {}. Keep both existed or both non-existed",
            ca_cert_path,
            ca_key_path
          ));
        } else if ca_cert_existed && ca_key_existed {
          let ca_cert = std::fs::read_to_string(ca_cert_path)?;
          let ca_key = std::fs::read_to_string(ca_key_path)?;
          (ca_cert, ca_key)
        } else {
          let (ca_cert, ca_key) = generate_ca_cert()?;
          std::fs::write(ca_cert_path, &ca_cert)?;
          std::fs::write(ca_key_path, &ca_key)?;
          (ca_cert, ca_key)
        }
      } else {
        return Err(anyhow::anyhow!(
          "CA certificate and key paths must be provided to store the generated CA certificate and key"
        ));
      };
      let subject_alt_names = vec!["localhost".to_string()];
      let (cert, key) = generate_signed_cert(&ca_cert, &ca_key, subject_alt_names)?;
      std::fs::write(cert_path, &cert)?;
      std::fs::write(key_path, &key)?;

      Ok((cert, key))
    } else {
      Err(anyhow::anyhow!(
        "Certificate and key paths must be provided and existed"
      ))
    }
  } else {
    Err(anyhow::anyhow!("Certificate and key paths must be provided"))
  }
}
