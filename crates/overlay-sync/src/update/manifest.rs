//! Le manifeste `latest.json` — format (§5 du plan de mise à jour), vérification de signature et
//! verdict « à jour / disponible / obligatoire ».
//!
//! Le format est celui que `xtask/src/dist.rs` produit ; les deux ne partagent pas de code (xtask
//! est hors du workspace, volontairement) mais le test `manifeste_de_xtask_dist` de ce module fige
//! un exemplaire réel pour que les deux restent d'accord.

use std::time::Duration;

use serde::Deserialize;

use super::{UpdateError, PLATFORM};

/// Délai propre à la lecture du manifeste : au démarrage, cinq secondes sans réponse valent
/// « vérification impossible » et l'overlay continue (§7 du plan). Le client `ureq` partagé de
/// `client.rs` a dix secondes ; ici on veut moins.
const MANIFEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema: u32,
    #[serde(default)]
    pub channel: String,
    pub version: String,
    #[serde(default)]
    pub commit: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub notes_url: Option<String>,
    #[serde(default)]
    pub minimum_version: Option<String>,
    #[serde(default)]
    pub assets: std::collections::BTreeMap<String, Asset>,
    /// Ignoré tant que la phase 3 n'est pas faite (décision 4 du plan) — lu quand même, pour ne
    /// pas rejeter un manifeste qui en porte.
    #[serde(default)]
    pub deltas: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub installed: Installed,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Installed {
    pub size: u64,
    pub sha256: String,
}

/// Verdict d'une vérification.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    UpToDate,
    Available {
        /// En boîte : le manifeste pèse bien plus que l'autre variante (lint `large_enum_variant`).
        manifest: Box<Manifest>,
        asset: Asset,
        mandatory: bool,
    },
}

impl Manifest {
    /// Décode et VÉRIFIE : la signature d'abord, le JSON ensuite — un manifeste dont la signature
    /// ne correspond pas n'est jamais parsé. `public_key` est le contenu d'un fichier `.pub`
    /// (deux lignes), `signature` celui d'un `.minisig`.
    pub fn parse_verified(
        json: &[u8],
        signature: &str,
        public_key: &str,
    ) -> Result<Self, UpdateError> {
        let pk = minisign_verify::PublicKey::decode(public_key.trim())
            .map_err(|e| UpdateError::Signature(format!("clé publique : {e}")))?;
        let sig = minisign_verify::Signature::decode(signature.trim())
            .map_err(|e| UpdateError::Signature(format!("signature : {e}")))?;
        pk.verify(json, &sig, false)
            .map_err(|e| UpdateError::Signature(e.to_string()))?;
        let manifest: Manifest =
            serde_json::from_slice(json).map_err(|e| UpdateError::Manifest(e.to_string()))?;
        if manifest.schema != 1 {
            return Err(UpdateError::Manifest(format!(
                "schéma {} inconnu (attendu 1)",
                manifest.schema
            )));
        }
        parse_version(&manifest.version)?;
        Ok(manifest)
    }

    /// L'asset de la plateforme de ce binaire.
    pub fn asset_for(&self, platform: &'static str) -> Result<&Asset, UpdateError> {
        self.assets
            .get(platform)
            .ok_or(UpdateError::NoAsset(platform))
    }

    /// Compare à la version courante (`build_info::VERSION`, `x.y.z`).
    pub fn verdict(&self, current: &str) -> Result<Verdict, UpdateError> {
        let current_v = parse_version(current)?;
        let latest_v = parse_version(&self.version)?;
        if latest_v <= current_v {
            return Ok(Verdict::UpToDate);
        }
        let asset = self.asset_for(PLATFORM)?.clone();
        let mandatory = match &self.minimum_version {
            Some(min) => current_v < parse_version(min)?,
            None => false,
        };
        Ok(Verdict::Available {
            manifest: Box::new(self.clone()),
            asset,
            mandatory,
        })
    }
}

pub fn parse_version(s: &str) -> Result<semver::Version, UpdateError> {
    semver::Version::parse(s.trim()).map_err(|e| UpdateError::Version(format!("{s} : {e}")))
}

/// Lit le manifeste et sa signature sur le réseau, vérifie, rend le verdict pour `current`.
pub fn check(current: &str) -> Result<Verdict, UpdateError> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(MANIFEST_TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    let json = fetch_small(&agent, &super::manifest_url(super::MANIFEST_NAME))?;
    let sig = fetch_small(&agent, &super::manifest_url(super::SIGNATURE_NAME))?;
    let sig = String::from_utf8(sig).map_err(|e| UpdateError::Signature(e.to_string()))?;
    let manifest = Manifest::parse_verified(&json, &sig, super::PUBLIC_KEY)?;
    manifest.verdict(current)
}

fn fetch_small(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, UpdateError> {
    let mut response = agent
        .get(url)
        .call()
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(UpdateError::Http {
            status,
            url: url.to_string(),
        });
    }
    response
        .body_mut()
        .read_to_vec()
        .map_err(|e| UpdateError::Network(e.to_string()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Une paire de clés jetable et une signature de `data` — le crate `minisign` (signature) n'est
    /// qu'une dépendance de TEST, le binaire n'embarque que la vérification.
    pub(crate) fn sign_for_test(data: &[u8]) -> (String, String) {
        let kp = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let sig = minisign::sign(None, &kp.sk, std::io::Cursor::new(data), None, None).unwrap();
        (kp.pk.to_box().unwrap().into_string(), sig.to_string())
    }

    /// Un `latest.json` tel que `xtask dist` l'écrit (sortie réelle du 2026-09-15).
    pub(crate) const SAMPLE: &str = r#"{
  "schema": 1,
  "channel": "stable",
  "version": "0.21.0",
  "commit": "abc1234",
  "publishedAt": "2026-09-15T14:54:47Z",
  "notesUrl": "https://github.com/Oumbra/wakfu-companion-overlay/releases/tag/v0.21.0",
  "minimumVersion": "0.20.0",
  "assets": {
    "linux-x86_64": {
      "name": "wakfu-companion-overlay-0.21.0-linux-x86_64.gz",
      "size": 3000495,
      "sha256": "0bd4fcaa0b065ba4fa8620b7ea974d2b33f214b01187929c12bfdf0f6f4b0b80",
      "installed": {
        "size": 3000002,
        "sha256": "5f4c9c46ee778febe05e02d4ae457990c595d556d848f1bc2deed77b4b97fd17"
      }
    },
    "windows-x86_64": {
      "name": "wakfu-companion-overlay-0.21.0-windows-x86_64.exe.gz",
      "size": 3000495,
      "sha256": "0bd4fcaa0b065ba4fa8620b7ea974d2b33f214b01187929c12bfdf0f6f4b0b80",
      "installed": {
        "size": 3000002,
        "sha256": "5f4c9c46ee778febe05e02d4ae457990c595d556d848f1bc2deed77b4b97fd17"
      }
    }
  },
  "deltas": {}
}
"#;

    #[test]
    fn manifeste_de_xtask_dist() {
        let (pk, sig) = sign_for_test(SAMPLE.as_bytes());
        let m = Manifest::parse_verified(SAMPLE.as_bytes(), &sig, &pk).unwrap();
        assert_eq!(m.version, "0.21.0");
        assert_eq!(m.minimum_version.as_deref(), Some("0.20.0"));
        assert_eq!(m.asset_for(PLATFORM).unwrap().installed.size, 3_000_002);
    }

    #[test]
    fn signature_invalide_ou_autre_cle_refusee() {
        let (pk, sig) = sign_for_test(SAMPLE.as_bytes());
        let mut altered = SAMPLE.to_string();
        altered = altered.replace("0.21.0", "9.9.9");
        assert!(matches!(
            Manifest::parse_verified(altered.as_bytes(), &sig, &pk),
            Err(UpdateError::Signature(_))
        ));
        let (autre_pk, _) = sign_for_test(b"autre");
        assert!(matches!(
            Manifest::parse_verified(SAMPLE.as_bytes(), &sig, &autre_pk),
            Err(UpdateError::Signature(_))
        ));
        // La clé publique réelle du dépôt ne valide pas non plus une signature de test.
        assert!(matches!(
            Manifest::parse_verified(SAMPLE.as_bytes(), &sig, super::super::PUBLIC_KEY),
            Err(UpdateError::Signature(_))
        ));
    }

    #[test]
    fn verdicts() {
        let (pk, sig) = sign_for_test(SAMPLE.as_bytes());
        let m = Manifest::parse_verified(SAMPLE.as_bytes(), &sig, &pk).unwrap();
        assert_eq!(m.verdict("0.21.0").unwrap(), Verdict::UpToDate);
        assert_eq!(m.verdict("0.22.3").unwrap(), Verdict::UpToDate);
        match m.verdict("0.20.4").unwrap() {
            Verdict::Available {
                mandatory, asset, ..
            } => {
                assert!(!mandatory);
                assert!(asset.name.contains(PLATFORM));
            }
            other => panic!("{other:?}"),
        }
        match m.verdict("0.19.0").unwrap() {
            Verdict::Available { mandatory, .. } => assert!(mandatory),
            other => panic!("{other:?}"),
        }
        assert!(m.verdict("n'importe quoi").is_err());
    }

    #[test]
    fn schema_inconnu_refuse() {
        let json = SAMPLE.replace("\"schema\": 1", "\"schema\": 2");
        let (pk, sig) = sign_for_test(json.as_bytes());
        assert!(matches!(
            Manifest::parse_verified(json.as_bytes(), &sig, &pk),
            Err(UpdateError::Manifest(_))
        ));
    }
}
