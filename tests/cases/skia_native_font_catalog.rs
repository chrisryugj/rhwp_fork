//! Legacy TextRun font lookup is independent from exact GlyphRun resource replay.

#![cfg(all(feature = "native-skia", not(target_arch = "wasm32")))]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use rhwp::paint::{CacheHint, GroupKind, LayerNode, PageLayerTree, PaintOp};
use rhwp::renderer::layer_renderer::LayerRasterRenderer;
use rhwp::renderer::render_tree::{BoundingBox, FieldMarkerType, TextRunNode};
use rhwp::renderer::skia::SkiaLayerRenderer;
use rhwp::renderer::TextStyle;

const TTF: &[u8] = include_bytes!("../fixtures/fonts/RHWPExactKerningSmoke.ttf");
const TTC: &[u8] = include_bytes!("../fixtures/fonts/RHWPExactFaceSmoke.ttc");

struct FontDirectory(PathBuf);

impl FontDirectory {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let serial = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rhwp-native-font-catalog-{}-{time}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for FontDirectory {
    fn drop(&mut self) {
        // This unique directory contains only fixtures created by this test.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn text_tree(text: &str, family: &str, bold: bool, italic: bool) -> PageLayerTree {
    let bounds = BoundingBox::new(0.0, 0.0, 100.0, 70.0);
    let run = TextRunNode {
        text: text.into(),
        style: TextStyle {
            font_family: family.into(),
            font_size: 40.0,
            bold,
            italic,
            ..Default::default()
        },
        char_shape_id: None,
        para_shape_id: None,
        section_index: None,
        para_index: None,
        char_start: None,
        cell_context: None,
        is_para_end: false,
        is_line_break_end: false,
        rotation: 0.0,
        is_vertical: false,
        char_overlap: None,
        border_fill_id: 0,
        baseline: 40.0,
        field_marker: FieldMarkerType::None,
        layout_positions: None,
        display_text: None,
    };
    PageLayerTree::new(
        bounds.width,
        bounds.height,
        LayerNode::group(
            bounds,
            None,
            vec![LayerNode::leaf(
                bounds,
                None,
                vec![PaintOp::text_run(
                    BoundingBox::new(10.0, 5.0, 80.0, 50.0),
                    run,
                )],
            )],
            CacheHint::StaticSubtree,
            GroupKind::Generic,
        ),
    )
}

fn render(renderer: &SkiaLayerRenderer, tree: &PageLayerTree) -> image::RgbaImage {
    let png = renderer.render_png(tree).unwrap();
    image::load_from_memory(&png).unwrap().to_rgba8()
}

fn text_run_mut(tree: &mut PageLayerTree) -> &mut TextRunNode {
    let rhwp::paint::LayerNodeKind::Group { children, .. } = &mut tree.root.kind else {
        unreachable!()
    };
    let rhwp::paint::LayerNodeKind::Leaf { ops } = &mut children[0].kind else {
        unreachable!()
    };
    let PaintOp::TextRun { run, .. } = &mut ops[0] else {
        unreachable!()
    };
    run
}

fn assert_same_pixels(actual: &image::RgbaImage, expected: &image::RgbaImage) {
    assert_eq!(actual.dimensions(), expected.dimensions());
    assert!(
        actual == expected,
        "first differing pixel: {:?}",
        actual
            .enumerate_pixels()
            .find(|(x, y, pixel)| pixel.0 != expected.get_pixel(*x, *y).0)
            .map(|(x, y, pixel)| (x, y, pixel.0, expected.get_pixel(x, y).0))
    );
}

fn table_offset(bytes: &[u8], tag: &[u8; 4]) -> usize {
    let count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    (0..count)
        .map(|index| 12 + index * 16)
        .find(|offset| &bytes[*offset..*offset + 4] == tag)
        .map(|offset| {
            u32::from_be_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()) as usize
        })
        .expect("fixture table must exist")
}

fn styled_font(weight: u16, width: u16, italic: bool, half_size: bool) -> Vec<u8> {
    // Keep the same licensed fixture outlines/cmap/family, but make each style's
    // ink visibly different by changing unitsPerEm. No external font is needed.
    let mut bytes = TTF.to_vec();
    let os2 = table_offset(&bytes, b"OS/2");
    bytes[os2 + 4..os2 + 6].copy_from_slice(&weight.to_be_bytes());
    bytes[os2 + 6..os2 + 8].copy_from_slice(&width.to_be_bytes());
    let bold = weight >= 600;
    let selection = if italic { 1u16 } else { 0 }
        | if bold { 1 << 5 } else { 0 }
        | if !bold && !italic { 1 << 6 } else { 0 };
    bytes[os2 + 62..os2 + 64].copy_from_slice(&selection.to_be_bytes());
    let head = table_offset(&bytes, b"head");
    let mac_style = u16::from(bold) | (u16::from(italic) << 1);
    bytes[head + 44..head + 46].copy_from_slice(&mac_style.to_be_bytes());
    if half_size {
        let upm = u16::from_be_bytes(bytes[head + 18..head + 20].try_into().unwrap());
        bytes[head + 18..head + 20].copy_from_slice(&(upm * 2).to_be_bytes());
    }
    let post = table_offset(&bytes, b"post");
    let angle = if italic { -12i32 * 65536 } else { 0 };
    bytes[post + 4..post + 8].copy_from_slice(&angle.to_be_bytes());
    let face = skia_safe::FontMgr::default()
        .new_from_data(&bytes, None)
        .expect("edited fixture must remain a usable typeface");
    let actual = face.font_style();
    assert_eq!(*actual.weight(), i32::from(weight));
    assert_eq!(*actual.width(), i32::from(width));
    assert_eq!(
        actual.slant() != skia_safe::font_style::Slant::Upright,
        italic
    );
    bytes
}

#[test]
fn native_legacy_text_catalog_loads_nonzero_ttc_faces_from_file_paths() {
    let fonts = FontDirectory::new();
    let path = fonts.write("two-faces.ttc", TTC);
    let renderer = SkiaLayerRenderer::new().with_font_paths(&[path]);
    let tree = text_tree("\u{e104}", "'rhwp exact face one'", false, false);
    let pixels = render(&renderer, &tree);
    assert!(pixels.pixels().any(|pixel| pixel[3] > 64 && pixel[0] < 128));
    let native = skia_safe::FontMgr::default()
        .new_from_data(TTC, Some(1))
        .unwrap();
    assert_ne!(native.unichar_to_glyph(0xe104), 0);
    let first = skia_safe::FontMgr::default()
        .new_from_data(TTC, Some(0))
        .unwrap();
    assert_eq!(first.unichar_to_glyph(0xe104), 0);
    assert!(tree.resources.font_resources().faces.is_empty());
    assert!(tree.resources.font_resources().blobs.is_empty());
    // An unsupported name has no glyph in any normal fallback font. Loading
    // face 0 alone used to leave this legacy TextRun empty.
    let unsupported = text_tree("\u{e104}", "no-such-rhwp-family", false, false);
    assert!(pixels != render(&renderer, &unsupported));
}

fn assert_style_choice(
    family: &str,
    first_style: (u16, u16, bool),
    preferred_style: (u16, u16, bool),
    bold: bool,
    italic: bool,
) {
    let fonts = FontDirectory::new();
    let first = fonts.write(
        "a-first.ttf",
        &styled_font(first_style.0, first_style.1, first_style.2, false),
    );
    let preferred = fonts.write(
        "b-preferred.ttf",
        &styled_font(
            preferred_style.0,
            preferred_style.1,
            preferred_style.2,
            true,
        ),
    );
    let tree = text_tree("A", family, bold, italic);
    let only_preferred = SkiaLayerRenderer::new().with_font_paths(&[preferred]);
    let both = SkiaLayerRenderer::new().with_font_paths(&[fonts.0.clone()]);
    let only_first = SkiaLayerRenderer::new().with_font_paths(&[first]);
    let expected = render(&only_preferred, &tree);
    assert_same_pixels(&render(&both, &tree), &expected);
    assert!(render(&only_first, &tree) != expected);
}

#[test]
fn native_legacy_text_catalog_matches_weight_without_double_synthetic_bold() {
    assert_style_choice(
        "RHWP Kerning Boundary",
        (400, 5, false),
        (700, 5, false),
        true,
        false,
    );
}

#[test]
fn native_legacy_text_catalog_matches_width() {
    assert_style_choice(
        "RHWP Kerning Boundary",
        (400, 3, false),
        (400, 5, false),
        false,
        false,
    );
}

#[test]
fn native_legacy_text_catalog_matches_slant() {
    assert_style_choice(
        "RHWP Kerning Boundary",
        (400, 5, false),
        (400, 5, true),
        false,
        true,
    );
}

#[test]
fn native_legacy_text_catalog_uses_shared_named_weight_hints() {
    assert_style_choice(
        "RHWP Kerning Boundary Light",
        (400, 5, false),
        (300, 5, false),
        false,
        false,
    );
    assert_style_choice(
        "RHWP Kerning Boundary Bold",
        (400, 5, false),
        (700, 5, false),
        false,
        false,
    );
}

#[test]
fn native_legacy_font_path_refresh_replaces_catalog_and_recorded_choices() {
    let fonts = FontDirectory::new();
    let first = fonts.write("a.ttf", &styled_font(400, 5, false, false));
    let second = fonts.write("b.ttf", &styled_font(400, 5, false, true));
    let tree = text_tree("A", "RHWP Kerning Boundary", false, false);
    let renderer = SkiaLayerRenderer::new().with_font_paths(&[first]);
    let first_pixels = render(&renderer, &tree);
    assert_same_pixels(&render(&renderer, &tree), &first_pixels);
    assert!(renderer.cache_stats().pictures.hits > 0);
    let renderer = renderer.with_font_paths(std::slice::from_ref(&second));
    assert_eq!(renderer.cache_stats().pictures.entries, 0);
    let refreshed = render(&renderer, &tree);
    assert!(refreshed != first_pixels);
    let fresh = SkiaLayerRenderer::new().with_font_paths(&[second]);
    assert_same_pixels(&refreshed, &render(&fresh, &tree));
}

#[test]
fn native_legacy_combining_shaping_preserves_layout_anchors_and_condensed_ratio() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ttfs/opensource/NotoSansKR-Regular.ttf");
    let bytes = std::fs::read(&path).unwrap();
    let manager = skia_safe::FontMgr::default();
    let face = manager.new_from_data(&bytes, None).unwrap();
    let shaper = skia_safe::Shaper::new_shape_then_wrap(manager).unwrap();
    let renderer = SkiaLayerRenderer::new().with_font_paths(&[path]);
    for (style_ratio, draw_size, draw_ratio) in [(1.0, 40.0, 1.0), (0.64, 32.0, 0.8)] {
        let mut tree = text_tree("A\u{301} B", "Noto Sans KR", false, false);
        let run = text_run_mut(&mut tree);
        run.style.ratio = style_ratio;
        run.layout_positions = Some(vec![0.0, 30.0, 30.0, 70.0, 95.0]);
        let mut font = skia_safe::Font::new(face.clone(), draw_size);
        font.set_edging(skia_safe::font::Edging::AntiAlias);
        let blob = shaper
            .shape_text_blob("A\u{301}", &font, true, 1_000_000.0, (0.0, 0.0))
            .unwrap()
            .0;
        let reference = |shaped: bool| {
            let mut surface = skia_safe::surfaces::raster_n32_premul((100, 70)).unwrap();
            let canvas = surface.canvas();
            canvas.clear(skia_safe::Color::TRANSPARENT);
            let mut paint = skia_safe::Paint::default();
            paint.set_anti_alias(true);
            paint.set_color(skia_safe::Color::BLACK);
            for (text, x) in [("A\u{301}", 10.0), ("B", 80.0)] {
                canvas.save();
                canvas.translate((x, 45.0));
                canvas.scale((draw_ratio, 1.0));
                if shaped && text != "B" {
                    canvas.draw_text_blob(&blob, (0.0, 0.0), &paint);
                } else {
                    canvas.draw_str(text, (0.0, 0.0), &font, &paint);
                }
                canvas.restore();
            }
            let png = skia_safe::png_encoder::encode_image(
                None::<&mut skia_safe::gpu::DirectContext>,
                &surface.image_snapshot(),
                &Default::default(),
            )
            .unwrap();
            image::load_from_memory(png.as_bytes()).unwrap().to_rgba8()
        };
        let expected = reference(true);
        assert!(
            expected != reference(false),
            "fixture must distinguish shaped replay"
        );
        assert_same_pixels(&render(&renderer, &tree), &expected);
        assert_same_pixels(&render(&renderer, &tree), &expected);
    }
}
