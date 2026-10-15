//! Bounded custom/bundled font catalogs, separate from system font priority.

use std::collections::HashMap;
use std::path::PathBuf;

use skia_safe::{Data, FontMgr, FontStyle, Typeface};

const MAX_FONT_FILES: usize = 256;
const MAX_FONT_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_FONT_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_COLLECTION_FACES: u32 = 64;

#[derive(Clone, Default)]
pub(super) struct TypefaceCatalog {
    families: HashMap<String, Vec<Typeface>>,
}

impl TypefaceCatalog {
    /// Load only the supplied byte snapshots. Unlike the path-based fallback
    /// loader, every face must parse and construct successfully or the whole
    /// replacement catalog is discarded. No environment paths are consulted.
    pub fn try_from_data(
        font_mgr: &FontMgr,
        sources: &[Vec<u8>],
    ) -> Result<Self, crate::error::HwpError> {
        use crate::error::HwpError;
        let invalid = |message: &str| HwpError::RenderError(message.into());
        if sources.len() > MAX_FONT_FILES {
            return Err(invalid("native font file count exceeds 256"));
        }
        let mut catalog = Self::default();
        let mut total_bytes = 0u64;
        let mut retained_bytes = 0u64;
        for bytes in sources {
            let length = bytes.len() as u64;
            if length == 0 || length > MAX_FONT_FILE_BYTES {
                return Err(invalid("native font size must be in 1..=32 MiB"));
            }
            total_bytes = total_bytes
                .checked_add(length)
                .ok_or_else(|| invalid("native font input byte count overflow"))?;
            if total_bytes > MAX_FONT_TOTAL_BYTES {
                return Err(invalid("native font input bytes exceed 64 MiB"));
            }
            let faces = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
            if faces == 0 || faces > MAX_COLLECTION_FACES {
                return Err(invalid(
                    "native font collection face count must be in 1..=64",
                ));
            }
            let retained = length
                .checked_mul(u64::from(faces))
                .and_then(|bytes| retained_bytes.checked_add(bytes))
                .ok_or_else(|| invalid("native font retained byte count overflow"))?;
            if retained > MAX_FONT_TOTAL_BYTES {
                return Err(invalid("native font retained copies exceed 64 MiB"));
            }
            retained_bytes = retained;
            for face_index in 0..faces {
                ttf_parser::Face::parse(bytes, face_index)
                    .map_err(|_| invalid("native font face could not be parsed"))?;
                let face = font_mgr
                    .new_from_data(Data::new_copy(bytes), Some(face_index))
                    .ok_or_else(|| invalid("native font face could not be constructed by Skia"))?;
                let family = face.family_name().trim().to_lowercase();
                if family.is_empty() {
                    return Err(invalid("native font face has no family name"));
                }
                catalog.families.entry(family).or_default().push(face);
            }
        }
        Ok(catalog)
    }

    pub fn load(font_mgr: &FontMgr, sources: &[PathBuf]) -> Self {
        let mut catalog = Self::default();
        let mut total_bytes = 0u64;
        let mut retained_bytes = 0u64;
        let mut loaded_files = 0usize;
        let mut seen = std::collections::HashSet::new();
        for source in sources {
            let mut paths = if source.is_file() {
                vec![source.clone()]
            } else {
                std::fs::read_dir(source)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|entry| entry.path())
                    .collect::<Vec<_>>()
            };
            paths.sort();
            for path in paths {
                let supported = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        matches!(
                            extension.to_ascii_lowercase().as_str(),
                            "ttf" | "otf" | "ttc"
                        )
                    });
                if !supported || !seen.insert(path.clone()) {
                    continue;
                }
                if loaded_files >= MAX_FONT_FILES {
                    return catalog;
                }
                let Ok(metadata) = path.metadata() else {
                    continue;
                };
                if !metadata.is_file()
                    || metadata.len() == 0
                    || metadata.len() > MAX_FONT_FILE_BYTES
                    || total_bytes.saturating_add(metadata.len()) > MAX_FONT_TOTAL_BYTES
                {
                    continue;
                }
                let Ok(bytes) = std::fs::read(&path) else {
                    continue;
                };
                if bytes.len() as u64 > MAX_FONT_FILE_BYTES
                    || total_bytes.saturating_add(bytes.len() as u64) > MAX_FONT_TOTAL_BYTES
                {
                    continue;
                }
                total_bytes += bytes.len() as u64;
                loaded_files += 1;
                let faces = ttf_parser::fonts_in_collection(&bytes)
                    .unwrap_or(1)
                    .min(MAX_COLLECTION_FACES);
                for face_index in 0..faces {
                    // Each face owns a fresh Data copy of the complete collection.
                    // Account retained copies, not just input files.
                    if retained_bytes.saturating_add(bytes.len() as u64) > MAX_FONT_TOTAL_BYTES {
                        break;
                    }
                    if ttf_parser::Face::parse(&bytes, face_index).is_err() {
                        continue;
                    }
                    if let Some(face) =
                        font_mgr.new_from_data(Data::new_copy(&bytes), Some(face_index))
                    {
                        retained_bytes += bytes.len() as u64;
                        catalog
                            .families
                            .entry(face.family_name().trim().to_lowercase())
                            .or_default()
                            .push(face);
                    }
                }
            }
        }
        catalog
    }

    pub fn candidates(&self, family: &str, style: FontStyle) -> Vec<Typeface> {
        let mut result = Vec::new();
        for family in family
            .split(',')
            .map(|family| family.trim().trim_matches(['\'', '"']))
        {
            if let Some(faces) = self.families.get(&family.to_lowercase()) {
                let mut faces = faces.clone();
                faces.sort_by_key(|face| {
                    let actual = face.font_style();
                    (
                        u8::from(actual.slant() != style.slant()),
                        (*actual.weight() - *style.weight()).abs(),
                        (*actual.width() - *style.width()).abs(),
                    )
                });
                result.extend(faces);
            }
        }
        result
    }
}
