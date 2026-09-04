//! Identité de fichier, indépendante du chemin — sert à distinguer « le même fichier a grandi »
//! de « un nouveau fichier a été créé au même chemin » (rotation), même quand la taille seule ne
//! permet pas de trancher (remplacement à taille croissante, voir docs/plan-architecture.md §5.2).
//!
//! **Limite connue, pas garantie par POSIX (2026-09-04, voir §5.2 du plan)** : après un `rm` suivi
//! d'un `create` au même chemin, RIEN ne garantit que l'OS alloue un inode différent de celui tout
//! juste libéré — constaté empiriquement ici (démontré non déterministe : reproductible seul,
//! disparaît selon quelle autre activité fichier a eu lieu juste avant dans le même process). C'est
//! pour ça que [`crate::tailer::Tailer`] ne s'appuie JAMAIS sur `FileIdentity` seule pour détecter
//! une rotation — voir son champ `identity_prefix` et son test d'intégration
//! `rotation_nouveau_fichier_meme_chemin_relit_depuis_zero`, qui couvrent ce cas précis avec un
//! second signal fiable (le contenu d'un fichier ne se réécrit jamais rétroactivement). Un ancien
//! test unitaire ici affirmait `assert_ne!` sur les identités après un tel remplacement — retiré
//! (2026-09-04) car il encodait cette hypothèse justement fausse, pas un comportement de notre
//! code ; voir git blame pour son contenu d'origine si besoin.

use std::fs::File;
use std::io;

/// Identité stable d'un fichier ouvert : `dev`/`ino` sous Unix, numéro de volume + `FileId`
/// (128 bits) sous Windows. Deux [`FileIdentity`] égales garantissent qu'il s'agit du même fichier
/// physique ; différentes, qu'il a été remplacé (rotation) même si le chemin n'a pas changé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileIdentity(Inner);

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Inner {
    dev: u64,
    ino: u64,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Inner {
    volume_serial: u64,
    file_id: u128,
}

impl FileIdentity {
    #[cfg(unix)]
    pub fn of(file: &File) -> io::Result<Self> {
        use std::os::unix::fs::MetadataExt;
        let meta = file.metadata()?;
        Ok(FileIdentity(Inner {
            dev: meta.dev(),
            ino: meta.ino(),
        }))
    }

    #[cfg(windows)]
    pub fn of(file: &File) -> io::Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Storage::FileSystem::{
            FileIdInfo, GetFileInformationByHandleEx, FILE_ID_INFO,
        };

        let handle = HANDLE(file.as_raw_handle());
        let mut info = FILE_ID_INFO::default();
        unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileIdInfo,
                &mut info as *mut FILE_ID_INFO as *mut core::ffi::c_void,
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )
        }
        .map_err(io::Error::other)?;

        let file_id = u128::from_le_bytes(info.FileId.Identifier);
        Ok(FileIdentity(Inner {
            volume_serial: info.VolumeSerialNumber,
            file_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn meme_fichier_meme_identite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.log");
        fs::write(&path, b"une ligne\n").unwrap();

        let f1 = File::open(&path).unwrap();
        let f2 = File::open(&path).unwrap();
        assert_eq!(
            FileIdentity::of(&f1).unwrap(),
            FileIdentity::of(&f2).unwrap()
        );
    }

    // `remplacement_identite_differente` (assert_ne! avant/après un `rm`+`create` au même chemin)
    // retiré le 2026-09-04 : il affirmait une garantie que l'OS ne donne pas (inode réutilisé
    // possible) — voir la doc de module pour le détail et où cette couverture vit désormais
    // (`Tailer`, pas `FileIdentity`).
}
