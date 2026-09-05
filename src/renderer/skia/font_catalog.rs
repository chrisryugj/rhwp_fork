//! Bounded custom/bundled font catalogs, separate from system font priority.

use std::collections::HashMap;
use std::path::PathBuf;

use skia_safe::{FontMgr, FontStyle, Typeface};

const MAX_FONT_FILES: usize = 256;
const MAX_FONT_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_FONT_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_COLLECTION_FACES: u32 = 64;

#[derive(Clone, Default)]
pub(super) struct TypefaceCatalog {
    families: HashMap<String, Vec<Typeface>>,
}

impl TypefaceCatalog {
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
                    // new_from_data owns a copy of the complete collection for
                    // each face. Account retained copies, not just input files.
                    if retained_bytes.saturating_add(bytes.len() as u64) > MAX_FONT_TOTAL_BYTES {
                        break;
                    }
                    if ttf_parser::Face::parse(&bytes, face_index).is_err() {
                        continue;
                    }
                    if let Some(face) = font_mgr.new_from_data(&bytes, Some(face_index as usize)) {
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
