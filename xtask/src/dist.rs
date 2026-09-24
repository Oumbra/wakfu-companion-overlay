//! `xtask dist` — assemble les assets d'une Release GitHub (docs/plan-mise-a-jour.md §5-§6,
//! phase 1) à partir des binaires déjà compilés par les jobs de build :
//!
//! 1. compresse chaque binaire en gzip (`flate2`, déjà dans l'arbre de l'overlay — pas de
//!    `zstd`, décision « pas de brique nouvelle ») sous le nom
//!    `wakfu-companion-overlay-{version}-{plateforme}[.exe].gz` ;
//! 2. calcule le SHA-256 de l'asset compressé ET du binaire décompressé (`installed.sha256`,
//!    celui que l'overlay vérifie juste avant de se remplacer) ;
//! 3. écrit `latest.json` (schéma §5 du plan — pas d'URL absolue, les assets se reconstruisent
//!    depuis `version` + `name`) ;
//! 4. le signe (`latest.json.minisig`) avec la clé privée lue dans `MINISIGN_SECRET_KEY` (contenu
//!    du fichier de clé, chiffré) déchiffrée EN MÉMOIRE par `MINISIGN_PASSWORD` — la clé n'est
//!    jamais écrite sur le disque du runner ; `--unsigned` n'existe que pour un essai local, une
//!    Release non signée ne doit jamais partir (le workflow ne passe pas ce drapeau). Avec
//!    `--public-key`, la signature est aussitôt **revérifiée** avec la clé publique commitée
//!    (`wakfu-overlay.pub`) : un secret qui ne correspondrait pas à cette clé — rotation à
//!    moitié faite, mauvais dépôt — fait échouer la publication ici plutôt que chez l'utilisateur ;
//! 5. **mesure** ce qu'aurait pesé un patch `bsdiff` depuis la Release précédente
//!    (`--previous DIR`, assets `.gz` téléchargés par le workflow) — décision 4 du plan : le
//!    différentiel n'est pas publié tant que ce chiffre, lu sur quelques Releases réelles, ne l'a
//!    pas justifié. Le rapport (`delta-measure.md`) est repris dans le résumé du job.
//!
//! Usage :
//! ```text
//! cargo run --manifest-path xtask/Cargo.toml --release -- dist \
//!     --version 0.21.0 --commit a1b2c3d \
//!     --asset windows-x86_64=target/release/wakfu-companion-overlay.exe \
//!     --asset linux-x86_64=target/release/wakfu-companion-overlay-x11 \
//!     --out dist [--previous previous/] [--min-version 0.17.0] [--notes-url URL] \
//!     [--public-key wakfu-overlay.pub] [--unsigned]
//! ```
//!
//! Aucune dépendance sur le workspace principal : ce module ne lit que des fichiers.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Nom de base de tous les assets — le nom du dépôt, pour qu'un fichier téléchargé se reconnaisse
/// seul dans un dossier de téléchargements.
const ASSET_STEM: &str = "wakfu-companion-overlay";
const MANIFEST_NAME: &str = "latest.json";
const MEASURE_REPORT_NAME: &str = "delta-measure.md";
const SCHEMA: u32 = 1;
const CHANNEL: &str = "stable";

pub fn run() {
    let args = match Args::parse(std::env::args().skip(2)) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("xtask dist : {message}\n{USAGE}");
            std::process::exit(2);
        }
    };
    if let Err(message) = build(&args) {
        eprintln!("xtask dist : {message}");
        std::process::exit(1);
    }
}

const USAGE: &str =
    "usage : xtask dist --version X.Y.Z --commit HASH --asset <plateforme>=<binaire> [--asset …] \
--out DIR [--previous DIR] [--min-version X.Y.Z] [--notes-url URL] [--public-key FICHIER.pub] [--unsigned]";

#[derive(Debug)]
struct Args {
    version: String,
    commit: String,
    assets: Vec<(String, PathBuf)>,
    out: PathBuf,
    previous: Option<PathBuf>,
    min_version: Option<String>,
    notes_url: Option<String>,
    public_key: Option<PathBuf>,
    unsigned: bool,
}

impl Args {
    fn parse(mut it: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut version = None;
        let mut commit = None;
        let mut assets = Vec::new();
        let mut out = None;
        let mut previous = None;
        let mut min_version = None;
        let mut notes_url = None;
        let mut public_key = None;
        let mut unsigned = false;
        let value = |it: &mut dyn Iterator<Item = String>, flag: &str| {
            it.next().ok_or_else(|| format!("{flag} attend une valeur"))
        };
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--version" => version = Some(value(&mut it, "--version")?),
                "--commit" => commit = Some(value(&mut it, "--commit")?),
                "--asset" => {
                    let spec = value(&mut it, "--asset")?;
                    let (platform, path) = spec.split_once('=').ok_or_else(|| {
                        format!("--asset attend <plateforme>=<chemin>, reçu {spec}")
                    })?;
                    assets.push((platform.to_string(), PathBuf::from(path)));
                }
                "--out" => out = Some(PathBuf::from(value(&mut it, "--out")?)),
                "--previous" => previous = Some(PathBuf::from(value(&mut it, "--previous")?)),
                "--min-version" => min_version = Some(value(&mut it, "--min-version")?),
                "--notes-url" => notes_url = Some(value(&mut it, "--notes-url")?),
                "--public-key" => public_key = Some(PathBuf::from(value(&mut it, "--public-key")?)),
                "--unsigned" => unsigned = true,
                other => return Err(format!("argument inconnu : {other}")),
            }
        }
        let version = version.ok_or("--version manquant")?;
        check_semver(&version)?;
        if let Some(min) = &min_version {
            check_semver(min)?;
        }
        if assets.is_empty() {
            return Err("au moins un --asset est requis".into());
        }
        Ok(Self {
            version,
            commit: commit.ok_or("--commit manquant")?,
            assets,
            out: out.ok_or("--out manquant")?,
            previous,
            min_version,
            notes_url,
            public_key,
            unsigned,
        })
    }
}

/// `x.y.z`, trois entiers — le seul format que `[workspace.package] version` produit
/// (`scripts/bump-version.sh`) et que l'overlay comparera (`semver`). Pas de pré-release ici.
fn check_semver(s: &str) -> Result<(), String> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    {
        Ok(())
    } else {
        Err(format!("version invalide : {s} (attendu x.y.z)"))
    }
}

// --- Manifeste (schéma §5 du plan) --------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema: u32,
    channel: &'static str,
    version: String,
    commit: String,
    published_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minimum_version: Option<String>,
    /// `BTreeMap` : ordre stable des plateformes, donc un `latest.json` reproductible.
    assets: BTreeMap<String, Asset>,
    /// Toujours vide en phase 1 (décision 4 du plan) — la clé existe déjà pour que l'overlay
    /// n'ait pas à changer de schéma le jour où les patchs sont publiés.
    deltas: BTreeMap<String, Vec<Delta>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Asset {
    name: String,
    size: u64,
    sha256: String,
    installed: Installed,
}

#[derive(Debug, Serialize)]
struct Installed {
    size: u64,
    sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Delta {
    from: String,
    from_sha256: String,
    name: String,
    size: u64,
    sha256: String,
}

fn asset_name(version: &str, platform: &str) -> String {
    let ext = if platform.starts_with("windows") {
        ".exe"
    } else {
        ""
    };
    format!("{ASSET_STEM}-{version}-{platform}{ext}.gz")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn gzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder
        .write_all(bytes)
        .map_err(|e| format!("gzip : {e}"))?;
    encoder.finish().map_err(|e| format!("gzip : {e}"))
}

fn gunzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = flate2::read::GzDecoder::new(bytes);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| format!("gunzip : {e}"))?;
    Ok(out)
}

/// Horodatage ISO 8601 UTC sans dépendance de date : `SystemTime` → calendrier civil (algorithme
/// de Howard Hinnant, `days_from_civil` inversé). Seconde de précision, largement suffisant pour
/// « publié le ».
fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

// --- Signature ----------------------------------------------------------------------------------

/// Signe `data` avec la clé privée `minisign` (contenu du fichier `.key`, chiffré) et son mot de
/// passe. Le contenu est normalisé (espaces et sauts de ligne en bordure retirés) : un secret
/// collé avec ou sans retour à la ligne final donne le même résultat.
fn sign(data: &[u8], secret_key_box: &str, password: &str) -> Result<String, String> {
    let sk = minisign::SecretKeyBox::from_string(secret_key_box.trim())
        .map_err(|e| format!("clé privée illisible : {e}"))?
        .into_secret_key(Some(password.to_string()))
        .map_err(|e| format!("clé privée : déchiffrement refusé ({e}) — mot de passe erroné ?"))?;
    let signature = minisign::sign(
        None,
        &sk,
        std::io::Cursor::new(data),
        Some(&format!("{ASSET_STEM} {MANIFEST_NAME}")),
        Some("signature: minisign secret key"),
    )
    .map_err(|e| format!("signature : {e}"))?;
    Ok(signature.to_string())
}

/// Vérifie une signature produite par [`sign`] avec le contenu d'un fichier `.pub` — ce que
/// l'overlay fera avec `minisign-verify` (phase 2), et ce que le workflow refait juste après avoir
/// signé, pour ne jamais publier un manifeste que la clé publique commitée ne validerait pas.
fn verify(data: &[u8], public_key_box: &str, signature_box: &str) -> Result<(), String> {
    let pk = minisign::PublicKeyBox::from_string(public_key_box.trim())
        .map_err(|e| format!("clé publique illisible : {e}"))?
        .into_public_key()
        .map_err(|e| format!("clé publique : {e}"))?;
    let sig = minisign::SignatureBox::from_string(signature_box)
        .map_err(|e| format!("signature illisible : {e}"))?;
    minisign::verify(&pk, &sig, std::io::Cursor::new(data), true, false, false)
        .map_err(|e| format!("signature invalide : {e}"))
}

// --- Mesure du différentiel (décision 4 du plan) ------------------------------------------------

struct DeltaMeasure {
    platform: String,
    previous_version: String,
    full_gz_size: u64,
    patch_size: u64,
}

/// Retrouve dans `previous/` l'asset de la même plateforme (`{stem}-{version}-{platform}[.exe].gz`)
/// et renvoie `(version, contenu décompressé)`.
fn previous_binary(previous: &Path, platform: &str) -> Result<Option<(String, Vec<u8>)>, String> {
    let entries = fs::read_dir(previous).map_err(|e| format!("{} : {e}", previous.display()))?;
    let suffix = asset_name("", platform);
    let suffix = suffix
        .trim_start_matches(&format!("{ASSET_STEM}-"))
        .to_string(); // "-{platform}[.exe].gz"
    let prefix = format!("{ASSET_STEM}-");
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(rest) = name.strip_prefix(&prefix) {
            if let Some(version) = rest.strip_suffix(&suffix) {
                if check_semver(version).is_ok() {
                    let bytes = fs::read(entry.path()).map_err(|e| format!("{name} : {e}"))?;
                    return Ok(Some((version.to_string(), gunzip(&bytes)?)));
                }
            }
        }
    }
    Ok(None)
}

fn measure_delta(source: &[u8], target: &[u8]) -> Result<u64, String> {
    qbsdiff::Bsdiff::new(source, target)
        .compare(std::io::sink())
        .map_err(|e| format!("bsdiff : {e}"))
}

fn megabytes(bytes: u64) -> String {
    if bytes < 1_000_000 {
        format!("{:.0} Ko", bytes as f64 / 1_000.0)
    } else {
        format!("{:.1} Mo", bytes as f64 / 1_000_000.0)
    }
}

fn measure_report(version: &str, measures: &[DeltaMeasure], skipped: &[String]) -> String {
    let mut out = format!("## Différentiel — mesure pour {version}\n\n");
    if measures.is_empty() && skipped.is_empty() {
        out.push_str(
            "Pas de Release précédente : aucune mesure possible (normal pour la première).\n",
        );
        return out;
    }
    out.push_str("| Plateforme | Depuis | Asset complet (gzip) | Patch bsdiff | Gain |\n|---|---|---|---|---|\n");
    for m in measures {
        let gain = if m.patch_size == 0 {
            0.0
        } else {
            m.full_gz_size as f64 / m.patch_size as f64
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | ×{gain:.1} |\n",
            m.platform,
            m.previous_version,
            megabytes(m.full_gz_size),
            megabytes(m.patch_size)
        ));
    }
    for platform in skipped {
        out.push_str(&format!(
            "| {platform} | — | — | — | asset précédent introuvable |\n"
        ));
    }
    out.push_str("\nSeuil retenu (docs/plan-mise-a-jour.md §9) : le différentiel n'est implémenté côté overlay qu'à partir d'un gain ×2.\n");
    out
}

// --- Assemblage ----------------------------------------------------------------------------------

fn build(args: &Args) -> Result<(), String> {
    fs::create_dir_all(&args.out).map_err(|e| format!("{} : {e}", args.out.display()))?;

    let mut assets = BTreeMap::new();
    let mut measures = Vec::new();
    let mut skipped = Vec::new();

    for (platform, path) in &args.assets {
        let binary = fs::read(path).map_err(|e| format!("{} : {e}", path.display()))?;
        let compressed = gzip(&binary)?;
        let name = asset_name(&args.version, platform);
        fs::write(args.out.join(&name), &compressed).map_err(|e| format!("{name} : {e}"))?;
        println!(
            "{name} : {} brut → {} gzip",
            megabytes(binary.len() as u64),
            megabytes(compressed.len() as u64)
        );
        if let Some(previous) = &args.previous {
            match previous_binary(previous, platform)? {
                Some((previous_version, previous_bin)) => {
                    let patch_size = measure_delta(&previous_bin, &binary)?;
                    println!(
                        "  delta depuis {previous_version} : patch {} (complet {} gzip)",
                        megabytes(patch_size),
                        megabytes(compressed.len() as u64)
                    );
                    measures.push(DeltaMeasure {
                        platform: platform.clone(),
                        previous_version,
                        full_gz_size: compressed.len() as u64,
                        patch_size,
                    });
                }
                None => skipped.push(platform.clone()),
            }
        }
        assets.insert(
            platform.clone(),
            Asset {
                name,
                size: compressed.len() as u64,
                sha256: sha256_hex(&compressed),
                installed: Installed {
                    size: binary.len() as u64,
                    sha256: sha256_hex(&binary),
                },
            },
        );
    }

    let manifest = Manifest {
        schema: SCHEMA,
        channel: CHANNEL,
        version: args.version.clone(),
        commit: args.commit.clone(),
        published_at: now_iso8601(),
        notes_url: args.notes_url.clone(),
        minimum_version: args.min_version.clone(),
        assets,
        deltas: BTreeMap::new(),
    };
    let mut json = serde_json::to_vec_pretty(&manifest).map_err(|e| format!("manifeste : {e}"))?;
    json.push(b'\n');
    fs::write(args.out.join(MANIFEST_NAME), &json).map_err(|e| format!("{MANIFEST_NAME} : {e}"))?;
    println!("{MANIFEST_NAME} : {} octets", json.len());

    if args.unsigned {
        println!("--unsigned : {MANIFEST_NAME} NON signé (essai local uniquement)");
    } else {
        let secret = std::env::var("MINISIGN_SECRET_KEY").map_err(|_| {
            "MINISIGN_SECRET_KEY absent (contenu du fichier de clé privée)".to_string()
        })?;
        let password = std::env::var("MINISIGN_PASSWORD")
            .map_err(|_| "MINISIGN_PASSWORD absent".to_string())?;
        let signature = sign(&json, &secret, &password)?;
        let sig_name = format!("{MANIFEST_NAME}.minisig");
        fs::write(args.out.join(&sig_name), &signature).map_err(|e| format!("{sig_name} : {e}"))?;
        println!("{sig_name} : signé");
        if let Some(public_key) = &args.public_key {
            let pk = fs::read_to_string(public_key)
                .map_err(|e| format!("{} : {e}", public_key.display()))?;
            verify(&json, &pk, &signature).map_err(|e| {
                format!(
                    "{e} — la clé privée du secret ne correspond pas à {}",
                    public_key.display()
                )
            })?;
            println!("{sig_name} : vérifié avec {}", public_key.display());
        }
    }

    if args.previous.is_some() {
        let report = measure_report(&args.version, &measures, &skipped);
        fs::write(args.out.join(MEASURE_REPORT_NAME), &report)
            .map_err(|e| format!("{MEASURE_REPORT_NAME} : {e}"))?;
        print!("{report}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keypair() -> (String, String, String) {
        let password = "essai".to_string();
        let kp = minisign::KeyPair::generate_encrypted_keypair(Some(password.clone())).unwrap();
        let sk_box = kp.sk.to_box(None).unwrap().into_string();
        let pk_box = kp.pk.to_box().unwrap().into_string();
        (sk_box, pk_box, password)
    }

    #[test]
    fn signature_valide_avec_ou_sans_saut_de_ligne_final() {
        let (sk, pk, password) = keypair();
        let data = b"{\"version\":\"1.2.3\"}\n";
        let sig = sign(data, &sk, &password).unwrap();
        verify(data, &pk, &sig).unwrap();
        // Secret collé sans retour à la ligne final (cas du mainteneur, 2026-09-15) : identique.
        let sig2 = sign(data, sk.trim_end(), &password).unwrap();
        verify(data, pk.trim_end(), &sig2).unwrap();
    }

    #[test]
    fn signature_rejetee_si_donnees_modifiees_ou_mot_de_passe_faux() {
        let (sk, pk, password) = keypair();
        let data = b"abc";
        let sig = sign(data, &sk, &password).unwrap();
        assert!(verify(b"abd", &pk, &sig).is_err());
        assert!(sign(data, &sk, "autre").is_err());
    }

    #[test]
    fn nom_des_assets_et_gzip_aller_retour() {
        assert_eq!(
            asset_name("0.21.0", "windows-x86_64"),
            "wakfu-companion-overlay-0.21.0-windows-x86_64.exe.gz"
        );
        assert_eq!(
            asset_name("0.21.0", "linux-x86_64"),
            "wakfu-companion-overlay-0.21.0-linux-x86_64.gz"
        );
        let payload: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        assert_eq!(gunzip(&gzip(&payload).unwrap()).unwrap(), payload);
    }

    #[test]
    fn manifeste_complet_et_mesure_du_delta() {
        let tmp = std::env::temp_dir().join(format!("xtask-dist-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let previous = tmp.join("previous");
        fs::create_dir_all(&previous).unwrap();
        // « Binaire » précédent : 200 Ko pseudo-aléatoires ; le nouveau en modifie 1 %.
        let mut old: Vec<u8> = Vec::with_capacity(200_000);
        let mut x: u32 = 12345;
        for _ in 0..200_000 {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            old.push((x >> 16) as u8);
        }
        let mut new = old.clone();
        for i in (0..new.len()).step_by(100) {
            new[i] ^= 0xFF;
        }
        fs::write(
            previous.join(asset_name("0.1.0", "linux-x86_64")),
            gzip(&old).unwrap(),
        )
        .unwrap();
        let bin = tmp.join("wakfu-companion-overlay-x11");
        fs::write(&bin, &new).unwrap();

        let args = Args::parse(
            [
                "--version",
                "0.2.0",
                "--commit",
                "abc1234",
                "--asset",
                &format!("linux-x86_64={}", bin.display()),
                "--out",
                &tmp.join("dist").display().to_string(),
                "--previous",
                &previous.display().to_string(),
                "--min-version",
                "0.1.0",
                "--unsigned",
            ]
            .iter()
            .map(|s| s.to_string()),
        )
        .unwrap();
        build(&args).unwrap();

        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(tmp.join("dist").join(MANIFEST_NAME)).unwrap())
                .unwrap();
        assert_eq!(manifest["schema"], 1);
        assert_eq!(manifest["version"], "0.2.0");
        assert_eq!(manifest["minimumVersion"], "0.1.0");
        let asset = &manifest["assets"]["linux-x86_64"];
        assert_eq!(
            asset["name"],
            "wakfu-companion-overlay-0.2.0-linux-x86_64.gz"
        );
        assert_eq!(asset["installed"]["size"], 200_000);
        assert_eq!(asset["installed"]["sha256"], sha256_hex(&new));
        let gz = fs::read(
            tmp.join("dist")
                .join("wakfu-companion-overlay-0.2.0-linux-x86_64.gz"),
        )
        .unwrap();
        assert_eq!(asset["sha256"], sha256_hex(&gz));
        assert_eq!(asset["size"], gz.len());
        assert!(manifest["deltas"].as_object().unwrap().is_empty());
        assert!(!tmp.join("dist").join("latest.json.minisig").exists());

        let report = fs::read_to_string(tmp.join("dist").join(MEASURE_REPORT_NAME)).unwrap();
        assert!(report.contains("| linux-x86_64 | 0.1.0 |"), "{report}");
        // 1 % de changement : le patch doit être nettement plus petit que l'asset complet.
        let patch = measure_delta(&old, &new).unwrap();
        assert!(
            patch < gz.len() as u64 / 2,
            "patch {patch} vs gzip {}",
            gz.len()
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn build_signe_et_revérifie_avec_la_clé_publique() {
        let (sk, pk, password) = keypair();
        let (_, autre_pk, _) = keypair();
        let tmp = std::env::temp_dir().join(format!("xtask-dist-sign-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("bin"), b"binaire").unwrap();
        fs::write(tmp.join("bonne.pub"), &pk).unwrap();
        fs::write(tmp.join("autre.pub"), &autre_pk).unwrap();
        std::env::set_var("MINISIGN_SECRET_KEY", &sk);
        std::env::set_var("MINISIGN_PASSWORD", &password);
        let args = |pub_name: &str| {
            Args::parse(
                [
                    "--version",
                    "0.2.0",
                    "--commit",
                    "abc1234",
                    "--asset",
                    &format!("linux-x86_64={}", tmp.join("bin").display()),
                    "--out",
                    &tmp.join("dist").display().to_string(),
                    "--public-key",
                    &tmp.join(pub_name).display().to_string(),
                ]
                .iter()
                .map(|s| s.to_string()),
            )
            .unwrap()
        };
        build(&args("bonne.pub")).unwrap();
        let json = fs::read(tmp.join("dist").join(MANIFEST_NAME)).unwrap();
        let sig = fs::read_to_string(tmp.join("dist").join("latest.json.minisig")).unwrap();
        verify(&json, &pk, &sig).unwrap();
        let err = build(&args("autre.pub")).unwrap_err();
        assert!(err.contains("ne correspond pas"), "{err}");
        std::env::remove_var("MINISIGN_SECRET_KEY");
        std::env::remove_var("MINISIGN_PASSWORD");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn arguments_invalides() {
        let parse = |a: &[&str]| Args::parse(a.iter().map(|s| s.to_string()));
        assert!(parse(&[
            "--version",
            "1.2",
            "--commit",
            "x",
            "--asset",
            "a=b",
            "--out",
            "o"
        ])
        .is_err());
        assert!(parse(&["--version", "1.2.3", "--commit", "x", "--out", "o"]).is_err());
        assert!(parse(&[
            "--version",
            "1.2.3",
            "--commit",
            "x",
            "--asset",
            "sans-egal",
            "--out",
            "o"
        ])
        .is_err());
        assert!(parse(&[
            "--version",
            "1.2.3",
            "--commit",
            "x",
            "--asset",
            "a=b",
            "--out",
            "o"
        ])
        .is_ok());
    }

    #[test]
    fn horodatage_iso8601() {
        let s = now_iso8601();
        assert_eq!(s.len(), 20, "{s}");
        assert!(s.starts_with("20") && s.ends_with('Z'), "{s}");
    }
}
