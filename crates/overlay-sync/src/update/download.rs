//! Téléchargement d'un asset **en flux** vers un fichier, avec progression — ce que
//! `client::fetch_bytes` ne sait pas faire (tout en mémoire, sans rappel).
//!
//! Le fichier est écrit sous `<dest>.part` puis renommé une fois complet : un `.part` orphelin
//! est un téléchargement interrompu, jamais un asset qu'on pourrait prendre pour valide. Le
//! SHA-256 est calculé au fil de l'eau et comparé à `expected_sha256` avant le renommage.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256};

use super::UpdateError;

/// Bloc de lecture : 64 Kio, et un rappel de progression par bloc.
const CHUNK: usize = 64 * 1024;

/// Pas de délai global (un téléchargement de 15 Mo dure ce qu'il dure), mais des délais de
/// connexion et de réception par bloc : un serveur qui ne répond plus finit par échouer, jamais
/// par figer le thread.
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(None)
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_body(Some(Duration::from_secs(60)))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Télécharge `url` dans `dest`, appelle `on_progress(reçus, total)` par bloc (`total` = en-tête
/// `Content-Length` si présent, sinon `expected_size`), vérifie le SHA-256 du fichier complet.
/// Renvoie la taille reçue.
pub fn fetch_to_file(
    url: &str,
    dest: &Path,
    expected_size: u64,
    expected_sha256: &str,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Result<u64, UpdateError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let part = dest.with_extension(match dest.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{ext}.part"),
        None => "part".to_string(),
    });

    let mut response = agent()
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
    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(expected_size);

    let mut reader = response.body_mut().as_reader();
    let mut file = fs::File::create(&part)?;
    let mut hasher = Sha256::new();
    let mut received: u64 = 0;
    let mut buf = vec![0u8; CHUNK];
    on_progress(0, total);
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| UpdateError::Network(e.to_string()))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
        received += n as u64;
        on_progress(received, total.max(received));
    }
    file.flush()?;
    drop(file);

    let digest = hex(&hasher.finalize());
    if !digest.eq_ignore_ascii_case(expected_sha256) {
        let _ = fs::remove_file(&part);
        return Err(UpdateError::HashMismatch {
            what: dest
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        });
    }
    fs::rename(&part, dest)?;
    Ok(received)
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;

    /// Un serveur HTTP d'une requête, sur un port libre — assez pour exercer le flux, la
    /// progression et le contrôle d'empreinte sans dépendre du réseau.
    fn serve_once(body: Vec<u8>, with_length: bool) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 0 {
                if line == "\r\n" {
                    break;
                }
                line.clear();
            }
            let mut stream = stream;
            let mut head = String::from("HTTP/1.1 200 OK\r\nConnection: close\r\n");
            if with_length {
                head.push_str(&format!("Content-Length: {}\r\n", body.len()));
            }
            head.push_str("\r\n");
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
            stream.flush().unwrap();
        });
        format!("http://{addr}/asset.gz")
    }

    #[test]
    fn telechargement_en_flux_avec_progression_et_empreinte() {
        let body: Vec<u8> = (0..300_000u32).map(|i| (i % 253) as u8).collect();
        let sha = hex(&Sha256::digest(&body));
        let url = serve_once(body.clone(), true);
        let dir = std::env::temp_dir().join(format!("overlay-update-dl-{}", std::process::id()));
        let dest = dir.join("asset.gz");
        let mut steps = Vec::new();
        let received = fetch_to_file(&url, &dest, 0, &sha, &mut |r, t| steps.push((r, t))).unwrap();
        assert_eq!(received, body.len() as u64);
        assert_eq!(fs::read(&dest).unwrap(), body);
        assert!(!dir.join("asset.gz.part").exists());
        assert_eq!(steps.first(), Some(&(0, body.len() as u64)));
        assert_eq!(steps.last(), Some(&(body.len() as u64, body.len() as u64)));
        assert!(steps.len() > 2, "progression par bloc attendue : {steps:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empreinte_fausse_efface_le_fichier_partiel() {
        let body = b"contenu".to_vec();
        let url = serve_once(body, false);
        let dir = std::env::temp_dir().join(format!("overlay-update-bad-{}", std::process::id()));
        let dest = dir.join("asset.gz");
        let err = fetch_to_file(&url, &dest, 7, "00", &mut |_, _| {}).unwrap_err();
        assert!(matches!(err, UpdateError::HashMismatch { .. }), "{err}");
        assert!(!dest.exists());
        assert!(!dir.join("asset.gz.part").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
