use std::fs::exists;

use anyhow::Result;
use rcgen::{
  BasicConstraints, CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, Ia5String, IsCa, KeyPair,
  KeyUsagePurpose, SanType,
};
use time::OffsetDateTime;

// pub fn generate_self_signed_cert(subject_alt_names: Vec<String>) -> Result<(String, String)> {
//   let signed = rcgen::generate_simple_self_signed(subject_alt_names)?;
//   let cert = signed.cert;
//   let key = signed.key_pair;

//   Ok((cert.pem(), key.serialize_pem()))
// }

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
  // let ca_cert = rustls_pemfile::certs(&mut ca_cert_pem.as_bytes())
  //   .next()
  //   .ok_or_else(|| anyhow::anyhow!("No certificate found in CA PEM"))??;

  let ca_key_pair = KeyPair::from_pem(ca_key_pem)?;

  params.subject_alt_names = subject_alt_names
    .into_iter()
    .filter_map(|name| Ia5String::try_from(name).map(|n| SanType::DnsName(n)).ok())
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

#[test]
fn test_generate_certs() {
  let (ca_cert, ca_key) = generate_ca_cert().unwrap();
  let subject_alt_names = vec!["localhost".to_string()];
  let (server_cert, server_key) = generate_signed_cert(&ca_cert, &ca_key, subject_alt_names).unwrap();
  assert!(!ca_cert.is_empty());
  assert!(!ca_key.is_empty());
  assert!(!server_cert.is_empty());
  assert!(!server_key.is_empty());

  println!("CA Cert:\n{}", ca_cert);
  println!("CA Key:\n{}", ca_key);
  println!("Server Cert:\n{}", server_cert);
  println!("Server Key:\n{}", server_key);
}

pub fn get_cert_from_file(
  cert_path: Option<String>, key_path: Option<String>, ca_cert_path: Option<String>, ca_key_path: Option<String>,
  allow_self_signed: bool,
) -> Result<(String, String)> {
  if cert_path.clone().map(|p| exists(p).is_ok_and(|inner| inner)).unwrap_or(false) &&
    key_path.clone().map(|p| exists(p).is_ok_and(|inner| inner)).unwrap_or(false)
  {
    let cert = std::fs::read_to_string(cert_path.unwrap())?;
    let key = std::fs::read_to_string(key_path.unwrap())?;
    Ok((cert, key))
  } else if allow_self_signed {
    let (ca_cert, ca_key) = if ca_cert_path.clone().map(|p| exists(p).is_ok_and(|inner| inner)).unwrap_or(false) &&
      ca_key_path.clone().map(|p| exists(p).is_ok_and(|inner| inner)).unwrap_or(false)
    {
      let ca_cert = std::fs::read_to_string(ca_cert_path.clone().unwrap())?;
      let ca_key = std::fs::read_to_string(ca_key_path.clone().unwrap())?;
      (ca_cert, ca_key)
    } else {
      generate_ca_cert()?
    };
    let subject_alt_names = vec!["localhost".to_string()];
    let (cert, key) = generate_signed_cert(&ca_cert, &ca_key, subject_alt_names)?;
    if let Some(cert_path) = cert_path {
      std::fs::write(cert_path, &cert)?;
    }
    if let Some(key_path) = key_path {
      std::fs::write(key_path, &key)?;
    }
    if let Some(ca_cert_path) = ca_cert_path {
      std::fs::write(ca_cert_path, &ca_cert)?;
    }
    if let Some(ca_key_path) = ca_key_path {
      std::fs::write(ca_key_path, &ca_key)?;
    }
    Ok((cert, key))
  } else {
    Err(anyhow::anyhow!(
      "Certificate and key paths are not provided or file path is not existed"
    ))
  }
}
