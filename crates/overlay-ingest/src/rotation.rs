//! Identité de fichier, indépendante du chemin — sert à distinguer « le même fichier a grandi »
//! de « un nouveau fichier a été créé au même chemin » (rotation), même quand la taille seule ne
//! permet pas de trancher (remplacement à taille croissante, voir docs/plan-architecture.md §5.2).

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
    use std::io::Write;

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

    #[test]
    fn remplacement_identite_differente() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.log");
        fs::write(&path, b"avant\n").unwrap();
        let before = FileIdentity::of(&File::open(&path).unwrap()).unwrap();

        // Remplacement (pas simple troncature) : nouveau fichier au même chemin.
        fs::remove_file(&path).unwrap();
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(b"apres\n").unwrap();
        let after = FileIdentity::of(&File::open(&path).unwrap()).unwrap();

        assert_ne!(before, after);
    }
}
