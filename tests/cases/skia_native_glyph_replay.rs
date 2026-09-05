//! Strict Native Skia alternatives must paint exact resources or retain TextRun.

#![cfg(all(feature = "native-skia", not(target_arch = "wasm32")))]

use std::io::Cursor;

use rhwp::paint::*;
use rhwp::renderer::layer_renderer::LayerRasterRenderer;
use rhwp::renderer::render_tree::{BoundingBox, FieldMarkerType, TextRunNode};
use rhwp::renderer::skia::{
    native_skia_glyph_run_replay_proof, NativeGlyphRunReplayProofReason, SkiaLayerRenderer,
};
use rhwp::renderer::{PathCommand, TextStyle};

const TTF: &[u8] = include_bytes!("../fixtures/fonts/RHWPExactKerningSmoke.ttf");
const TTC: &[u8] = include_bytes!("../fixtures/fonts/RHWPExactFaceSmoke.ttc");
const VARIABLE: &[u8] =
    include_bytes!("../../ttfs/redistributable/happiness-sans/HappinessSansVF.ttf");

fn fallback_tree() -> PageLayerTree {
    let run = TextRunNode {
        text: "A".into(),
        style: TextStyle {
            font_family: "sans-serif".into(),
            font_size: 32.0,
            color: 0x0000ff,
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
        baseline: 32.0,
        field_marker: FieldMarkerType::None,
        layout_positions: None,
        display_text: None,
    };
    PageLayerTree::new(
        180.0,
        80.0,
        LayerNode::leaf(
            BoundingBox::new(0.0, 0.0, 180.0, 80.0),
            None,
            vec![PaintOp::text_run(
                BoundingBox::new(120.0, 5.0, 40.0, 40.0),
                run,
            )],
        ),
    )
}

fn source(tree: &PageLayerTree) -> TextSourceSpan {
    let LayerNodeKind::Leaf { ops } = &tree.root.kind else {
        unreachable!()
    };
    let PaintOp::TextRun {
        source: Some(source),
        ..
    } = &ops[0]
    else {
        unreachable!()
    };
    source.clone()
}

fn ops_mut(tree: &mut PageLayerTree) -> &mut Vec<PaintOp> {
    let LayerNodeKind::Leaf { ops } = &mut tree.root.kind else {
        unreachable!()
    };
    ops
}

fn variant(kind: TextVariantKind) -> PaintVariantMeta {
    let mut variant = PaintVariantMeta::text_run_default("text-0");
    variant.variant_id = match kind {
        TextVariantKind::GlyphRun => "glyphRun",
        _ => "glyphOutline",
    }
    .into();
    variant.variant_kind = kind;
    variant.is_default_fallback = false;
    variant.quality = Some(TextVariantQuality::Exact);
    variant.anchor_op_id = Some("text-0".into());
    variant
}

fn diagnostics() -> GlyphRunDiagnostics {
    GlyphRunDiagnostics {
        quality: TextVariantQuality::Exact,
        replay_eligibility: GlyphRunReplayEligibility::Portable,
        strict_visual_eligible: true,
        max_origin_delta_px: 0.0,
        max_advance_delta_px: 0.0,
        max_residual_after_adjustment_px: 0.0,
        cluster_mismatch_count: 0,
        missing_glyph_count: 0,
        used_fallback_font_count: 0,
        reason: None,
    }
}

fn glyph_tree(bytes: &[u8], face_index: u32, character: char) -> PageLayerTree {
    let mut tree = fallback_tree();
    let digest = FontDigest {
        algorithm: "blake3".into(),
        value: resource_digest_hex(bytes),
    };
    let data_ref = BinaryResourceRef {
        kind: BinaryResourceKind::FontBlob,
        id: font_blob_resource_key(bytes.len(), &digest.value),
    };
    tree.resources.intern_font_blob_bytes(bytes);
    tree.resources
        .font_resources_mut()
        .blobs
        .push(FontBlobResource {
            id: FontBlobKey("blob".into()),
            digest: Some(digest.clone()),
            source: FontResourceSource::Embedded,
            data_ref: Some(data_ref.clone()),
            portability: FontPortability::PortableBlob { digest, data_ref },
        });
    tree.resources
        .font_resources_mut()
        .faces
        .push(FontFaceResource {
            id: FontFaceKey("face".into()),
            blob_key: FontBlobKey("blob".into()),
            face_index,
            postscript_name: None,
            family_names: Vec::new(),
            style_names: Vec::new(),
            weight_class: None,
            width_class: None,
            italic: None,
        });
    let glyph = ttf_parser::Face::parse(bytes, face_index)
        .unwrap()
        .glyph_index(character)
        .unwrap()
        .0;
    let run = LayerGlyphRunPaint {
        source: source(&tree),
        variant: variant(TextVariantKind::GlyphRun),
        paint_style: PaintTextStyle::from(&TextStyle {
            font_size: 32.0,
            ..Default::default()
        }),
        shape_key: ShapeKey {
            font_instance: FontInstanceKey {
                face_key: FontFaceKey("face".into()),
                size_px: 32.0,
                variations: Vec::new(),
                synthetic_bold: false,
                synthetic_italic: false,
            },
            direction: TextDirection::Ltr,
            writing_mode: WritingMode::HorizontalTb,
            script: None,
            language: None,
            features: Vec::new(),
            shaping_engine: ShapingEngineId("fixture".into()),
            fallback_policy: FontFallbackPolicyId("none".into()),
        },
        placement: TextRunPlacement {
            run_to_page: LayerAffineTransform {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: 10.0,
                f: 42.0,
            },
            baseline_y: 42.0,
        },
        glyph_ids: vec![u32::from(glyph)],
        positions: vec![LayerPoint { x: 0.0, y: 0.0 }],
        advances: None,
        clusters: vec![GlyphCluster {
            source_range_utf8: TextSourceRange::new(0, 1),
            source_range_utf16: Some(TextSourceRange::new(0, 1)),
            text_range_utf8: Some(TextSourceRange::new(0, 1)),
            glyph_range: GlyphRange::new(0, 1),
            flags: Vec::new(),
        }],
        direction: TextDirection::Ltr,
        bidi_level: Some(0),
        writing_mode: WritingMode::HorizontalTb,
        orientation: GlyphRunOrientation::Horizontal,
        glyph_transforms: None,
        diagnostics: diagnostics(),
    };
    ops_mut(&mut tree).push(PaintOp::glyph_run(
        BoundingBox::new(10.0, 10.0, 40.0, 40.0),
        run,
    ));
    tree
}

fn glyph_mut(tree: &mut PageLayerTree) -> &mut LayerGlyphRunPaint {
    let PaintOp::GlyphRun { run, .. } = &mut ops_mut(tree)[1] else {
        unreachable!()
    };
    run
}

fn proof(tree: &PageLayerTree) -> rhwp::renderer::skia::NativeGlyphRunReplayProof {
    let LayerNodeKind::Leaf { ops } = &tree.root.kind else {
        unreachable!()
    };
    let PaintOp::GlyphRun { run, .. } = &ops[1] else {
        unreachable!()
    };
    native_skia_glyph_run_replay_proof(run, &tree.resources)
}

fn render(tree: &PageLayerTree) -> image::RgbaImage {
    let png = SkiaLayerRenderer::new().render_png(tree).unwrap();
    image::load_from_memory(&png).unwrap().to_rgba8()
}

fn assert_same_pixels(actual: image::RgbaImage, expected: image::RgbaImage) {
    assert_eq!(actual.dimensions(), expected.dimensions());
    if let Some(((x, y, actual), expected)) = actual
        .enumerate_pixels()
        .zip(expected.pixels())
        .find(|((_, _, actual), expected)| *actual != *expected)
    {
        panic!("first differing pixel ({x}, {y}): {actual:?}, expected {expected:?}");
    }
}

fn assert_selected(tree: &PageLayerTree) -> image::RgbaImage {
    let image = render(tree);
    assert!(
        image
            .enumerate_pixels()
            .any(|(x, _, pixel)| x < 100 && pixel[3] > 64),
        "exact resource must paint visible left-side ink"
    );
    assert!(
        !image
            .enumerate_pixels()
            .any(|(x, _, pixel)| x >= 100 && pixel[3] > 64),
        "anchored right-side fallback must be suppressed"
    );
    image
}

#[test]
fn native_glyph_replay_constructs_ttf_and_exact_nonzero_ttc_face() {
    for tree in [glyph_tree(TTF, 0, 'A'), glyph_tree(TTC, 1, '\u{e104}')] {
        assert!(proof(&tree).typeface_constructible);
        assert_selected(&tree);
    }
}

#[test]
fn native_glyph_replay_changes_ink_for_exact_synthetic_instances() {
    let normal = glyph_tree(TTF, 0, 'A');
    let normal_pixels = assert_selected(&normal);
    for (bold, italic) in [(true, false), (false, true), (true, true)] {
        let mut tree = normal.clone();
        let instance = &mut glyph_mut(&mut tree).shape_key.font_instance;
        instance.synthetic_bold = bold;
        instance.synthetic_italic = italic;
        assert!(proof(&tree).typeface_constructible);
        assert!(normal_pixels != assert_selected(&tree));
    }
}

#[test]
fn native_glyph_replay_preserves_shadow_outline_and_relief_passes() {
    let normal = glyph_tree(TTF, 0, 'A');
    let normal_pixels = assert_selected(&normal);
    let mut images = Vec::new();
    for effect in 0..5 {
        let mut tree = normal.clone();
        let style = &mut glyph_mut(&mut tree).paint_style;
        match effect {
            0 | 4 => {
                style.shadow_type = 1;
                style.shadow_color = 0xff0000;
                style.shadow_offset_x = 8.0;
                style.shadow_offset_y = 5.0;
                if effect == 4 {
                    style.outline_type = 1;
                }
            }
            1 => style.outline_type = 1,
            2 => style.emboss = true,
            _ => style.engrave = true,
        }
        assert!(proof(&tree).typeface_constructible, "effect {effect}");
        let image = assert_selected(&tree);
        assert!(normal_pixels != image, "effect {effect}");
        if effect == 0 || effect == 4 {
            assert!(image.pixels().any(|pixel| pixel[2] > 200 && pixel[0] < 30));
        }
        images.push(image);
    }
    assert!(images[1] != images[4], "outline must retain its shadow");
    assert!(images[2] != images[3], "relief direction must change");
    let mut relief = normal.clone();
    glyph_mut(&mut relief).paint_style.emboss = true;
    let expected = assert_selected(&relief);
    let style = &mut glyph_mut(&mut relief).paint_style;
    style.engrave = true;
    style.outline_type = 1;
    style.shadow_type = 1;
    style.shadow_offset_x = 8.0;
    style.shadow_offset_y = 5.0;
    assert_same_pixels(assert_selected(&relief), expected);
    for offset in [f64::NAN, f64::INFINITY, f64::MAX] {
        let mut tree = normal.clone();
        let style = &mut glyph_mut(&mut tree).paint_style;
        style.shadow_type = 1;
        style.shadow_offset_x = offset;
        assert!(proof(&tree)
            .reasons
            .contains(&NativeGlyphRunReplayProofReason::UnsupportedPaintEffect));
        assert_same_pixels(render(&tree), render(&fallback_tree()));
    }
}

#[test]
fn native_glyph_replay_constructs_requested_variable_axis_and_rejects_invalid_tuples() {
    let normal = glyph_tree(VARIABLE, 0, '가');
    let axis = ttf_parser::Face::parse(VARIABLE, 0)
        .unwrap()
        .variation_axes()
        .into_iter()
        .find(|axis| axis.tag == ttf_parser::Tag::from_bytes(b"wght"))
        .unwrap();
    let mut images = Vec::new();
    for value in [axis.min_value, axis.def_value, axis.max_value] {
        let mut tree = normal.clone();
        glyph_mut(&mut tree).shape_key.font_instance.variations = vec![VariationAxisValue {
            tag: "wght".into(),
            value,
        }];
        assert!(proof(&tree).typeface_constructible);
        images.push(assert_selected(&tree));
    }
    assert!(images.first() != images.last());
    for axes in [
        vec![VariationAxisValue {
            tag: "wght".into(),
            value: axis.max_value + 1.0,
        }],
        vec![VariationAxisValue {
            tag: "nope".into(),
            value: 0.0,
        }],
        vec![VariationAxisValue {
            tag: "wght".into(),
            value: f32::NAN,
        }],
        vec![
            VariationAxisValue {
                tag: "wght".into(),
                value: axis.def_value
            };
            2
        ],
    ] {
        let mut tree = normal.clone();
        glyph_mut(&mut tree).shape_key.font_instance.variations = axes;
        assert!(proof(&tree)
            .reasons
            .contains(&NativeGlyphRunReplayProofReason::FontVariationUnsupported));
        assert_same_pixels(render(&tree), render(&fallback_tree()));
    }
}

#[test]
fn native_glyph_replay_rejects_invalid_bytes_digest_face_and_geometry_without_font_lookup() {
    let base = glyph_tree(TTF, 0, 'A');
    let fallback = render(&fallback_tree());
    for case in 0..7 {
        let mut tree = base.clone();
        match case {
            0 => {
                tree.resources.font_resources_mut().blobs[0]
                    .digest
                    .as_mut()
                    .unwrap()
                    .value = "wrong".into()
            }
            1 => tree.resources.font_resources_mut().faces[0].face_index = 99,
            2 => glyph_mut(&mut tree).glyph_ids[0] = u32::from(u16::MAX) + 1,
            3 => glyph_mut(&mut tree).positions[0].x = f64::MAX,
            4 => glyph_mut(&mut tree).shape_key.font_instance.size_px = f64::NAN,
            5 => glyph_mut(&mut tree).direction = TextDirection::Rtl,
            _ => {
                let face = tree.resources.font_resources().faces[0].clone();
                tree.resources.font_resources_mut().faces.push(face);
            }
        }
        assert!(!proof(&tree).typeface_constructible, "case {case}");
        assert_same_pixels(render(&tree), fallback.clone());
    }
}

fn rectangle_path() -> LayerGlyphOutlinePath {
    LayerGlyphOutlinePath {
        glyph_id: 1,
        source_range_utf8: TextSourceRange::new(0, 1),
        glyph_range: GlyphRange::new(0, 1),
        fill_rule: GlyphOutlineFillRule::NonZero,
        commands: vec![
            PathCommand::MoveTo(10.0, 10.0),
            PathCommand::LineTo(40.0, 10.0),
            PathCommand::LineTo(40.0, 40.0),
            PathCommand::LineTo(10.0, 40.0),
            PathCommand::ClosePath,
        ],
    }
}

fn outline_tree() -> PageLayerTree {
    let mut tree = fallback_tree();
    let outline = LayerGlyphOutlinePaint {
        source: source(&tree),
        variant: variant(TextVariantKind::GlyphOutline),
        payload_kind: GlyphOutlinePayloadKind::MonochromeFill,
        color_layers: None,
        bitmap_glyph: None,
        svg_glyph: None,
        paint_style: PaintTextStyle::from(&TextStyle::default()),
        placement: TextRunPlacement {
            run_to_page: LayerAffineTransform {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: 0.0,
                f: 0.0,
            },
            baseline_y: 0.0,
        },
        paths: vec![rectangle_path()],
        stroke: None,
        diagnostics: diagnostics(),
    };
    ops_mut(&mut tree).push(PaintOp::glyph_outline(
        BoundingBox::new(10.0, 10.0, 30.0, 30.0),
        outline,
    ));
    tree
}

fn outline_mut(tree: &mut PageLayerTree) -> &mut LayerGlyphOutlinePaint {
    let PaintOp::GlyphOutline { outline, .. } = &mut ops_mut(tree)[1] else {
        unreachable!()
    };
    outline
}

#[test]
fn native_glyph_replay_renders_outline_fill_rule_transform_and_stroke_order() {
    let mut tree = outline_tree();
    outline_mut(&mut tree).placement.run_to_page.e = 5.0;
    let image = assert_selected(&tree);
    assert_eq!(image.get_pixel(20, 20).0, [0, 0, 0, 255]);
    assert_eq!(image.get_pixel(12, 20)[3], 0);
    for order in [
        GlyphOutlinePaintOrder::FillThenStroke,
        GlyphOutlinePaintOrder::StrokeThenFill,
    ] {
        let outline = outline_mut(&mut tree);
        outline.payload_kind = GlyphOutlinePayloadKind::MonochromeFillStroke;
        outline.stroke = Some(GlyphOutlineStrokeStyle {
            color: 0x0000ff,
            width: 8.0,
            join: GlyphOutlineStrokeJoin::Miter,
            cap: GlyphOutlineStrokeCap::Butt,
            miter_limit: 4.0,
            paint_order: order,
        });
        let image = assert_selected(&tree);
        let inside_edge = image.get_pixel(17, 20);
        assert_eq!(
            inside_edge[0] > 200,
            order == GlyphOutlinePaintOrder::FillThenStroke
        );
    }
}

#[test]
fn native_glyph_replay_requires_every_variant_part_and_every_path() {
    let mut tree = outline_tree();
    let outline = outline_mut(&mut tree);
    outline.variant.part_count = 2;
    let mut second = outline.clone();
    second.variant.part_index = 1;
    second.paths[0].commands[0] = PathCommand::MoveTo(f64::NAN, 0.0);
    ops_mut(&mut tree).push(PaintOp::glyph_outline(
        BoundingBox::new(10.0, 10.0, 30.0, 30.0),
        second,
    ));
    assert_same_pixels(render(&tree), render(&fallback_tree()));
    ops_mut(&mut tree).pop();
    let error = SkiaLayerRenderer::new().render_png(&tree).unwrap_err();
    assert!(error.to_string().contains("has 1 parts, expected 2"));
}

fn bitmap_tree(bytes: &[u8]) -> PageLayerTree {
    let mut tree = outline_tree();
    let image_ref = tree.resources.intern_image_bytes(bytes);
    let outline = outline_mut(&mut tree);
    outline.payload_kind = GlyphOutlinePayloadKind::BitmapGlyph;
    outline.paths.clear();
    outline.bitmap_glyph = Some(BitmapGlyphPayload {
        image_ref,
        source_range_utf8: TextSourceRange::new(0, 1),
        glyph_range: GlyphRange::new(0, 1),
        placement: BoundingBox::new(25.0, 15.0, 20.0, 20.0),
        alpha_premultiplied: true,
        scaling_policy: BitmapGlyphScalingPolicy::SourceExact,
        filtering: BitmapGlyphFiltering::Nearest,
        transform_to_run: None,
    });
    tree
}

#[test]
fn native_glyph_replay_decodes_bitmap_before_selection_and_preserves_payload_placement() {
    let mut encoded = Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 255, 255]))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let tree = bitmap_tree(encoded.get_ref());
    let image = assert_selected(&tree);
    assert_eq!(image.get_pixel(30, 20).0, [0, 0, 255, 255]);
    assert_eq!(image.get_pixel(12, 12)[3], 0);
    let corrupt = &encoded.get_ref()[..encoded.get_ref().len() / 2];
    assert_same_pixels(render(&bitmap_tree(corrupt)), render(&fallback_tree()));
}

fn svg_tree(fragment: &str) -> PageLayerTree {
    let mut tree = outline_tree();
    let svg_ref = tree.resources.intern_svg_fragment(fragment);
    let outline = outline_mut(&mut tree);
    outline.payload_kind = GlyphOutlinePayloadKind::SvgGlyph;
    outline.paths.clear();
    outline.svg_glyph = Some(SvgGlyphPayload {
        svg_ref,
        source_range_utf8: TextSourceRange::new(0, 1),
        glyph_range: GlyphRange::new(0, 1),
        view_box: BoundingBox::new(10.0, 20.0, 30.0, 30.0),
        font_em_box: None,
        intrinsic_size: None,
        static_sanitized: true,
        script_allowed: false,
        animation_allowed: false,
        external_resources_allowed: false,
        interactivity_allowed: false,
        transform_to_run: None,
    });
    tree
}

#[test]
fn native_glyph_replay_uses_static_svg_viewbox_and_rejects_unsafe_resources() {
    let tree = svg_tree("<svg xmlns=\"http://www.w3.org/2000/svg\"><path fill=\"#0000ff\" d=\"M10 20H40V50H10Z\"/></svg>");
    let image = assert_selected(&tree);
    assert_eq!(image.get_pixel(20, 20).0, [0, 0, 255, 255]);
    for fragment in [
        "<svg><script>alert(1)</script><path d=\"M10 20H40V50Z\"/></svg>",
        "<svg><image href=\"file:///private/missing.png\"/><path d=\"M10 20H40V50Z\"/></svg>",
        "<svg><path d=\"Mbroken\"/></svg>",
    ] {
        assert_same_pixels(render(&svg_tree(fragment)), render(&fallback_tree()));
    }
}

#[test]
fn native_glyph_replay_renders_resolved_colrv0_layers_in_order() {
    let mut tree = outline_tree();
    let outline = outline_mut(&mut tree);
    outline.payload_kind = GlyphOutlinePayloadKind::ColorLayers;
    outline.paths.clear();
    let layer = |rgba| ColorLayerNode {
        layer_index: Some(0),
        glyph_id: Some(1),
        glyph_range: Some(GlyphRange::new(0, 1)),
        source_range_utf8: Some(TextSourceRange::new(0, 1)),
        source_font_ref: None,
        commands: Some(rectangle_path().commands),
        fill: Some(ResolvedColor {
            color_space: Some("sRGB".into()),
            rgba,
        }),
        fill_rule: Some(GlyphOutlineFillRule::NonZero),
        palette_index: None,
        color: None,
        opacity: None,
        transform_to_run: None,
    };
    outline.color_layers = Some(ColorLayersPayload {
        color_format: ColorGlyphFormat::ColrV0,
        source_font_ref: None,
        palette_ref: None,
        layers: vec![layer([1.0, 0.0, 0.0, 1.0]), layer([0.0, 0.0, 1.0, 0.5])],
        paint_graph: None,
        source_range_utf8: None,
        glyph_range: None,
    });
    let image = assert_selected(&tree);
    let pixel = image.get_pixel(20, 20);
    assert!((120..=135).contains(&pixel[0]) && (120..=135).contains(&pixel[2]));
    outline_mut(&mut tree).color_layers.as_mut().unwrap().layers[1]
        .fill
        .as_mut()
        .unwrap()
        .rgba[0] = f32::NAN;
    assert_same_pixels(render(&tree), render(&fallback_tree()));
}

fn static_tree(mut tree: PageLayerTree) -> PageLayerTree {
    tree.root = LayerNode::group(
        tree.root.bounds,
        None,
        vec![tree.root],
        CacheHint::StaticSubtree,
        GroupKind::Generic,
    );
    tree
}

fn cached_render(
    renderer: &SkiaLayerRenderer,
    tree: &PageLayerTree,
    scale: f64,
) -> image::RgbaImage {
    let options = rhwp::renderer::layer_renderer::RasterRenderOptions {
        scale,
        ..Default::default()
    };
    let png = renderer.render_png_with_options(tree, options).unwrap();
    image::load_from_memory(&png).unwrap().to_rgba8()
}

#[test]
fn native_cache_static_hits_match_direct_replay_and_invalidate_options_and_geometry() {
    let uncached = SkiaLayerRenderer::new().with_cache_limits(0, 0);
    let renderer = SkiaLayerRenderer::new();
    let mut tree = static_tree(glyph_tree(TTF, 0, 'A'));
    let expected = cached_render(&uncached, &tree, 1.0);
    assert_same_pixels(cached_render(&renderer, &tree, 1.0), expected.clone());
    assert_same_pixels(cached_render(&renderer, &tree, 1.0), expected);
    assert!(renderer.cache_stats().pictures.hits > 0);
    let mut misses = renderer.cache_stats().pictures.misses;
    for scale in [1.5, 2.0] {
        assert_same_pixels(
            cached_render(&renderer, &tree, scale),
            cached_render(&uncached, &tree, scale),
        );
        assert!(renderer.cache_stats().pictures.misses > misses);
        misses = renderer.cache_stats().pictures.misses;
    }
    tree.profile = RenderProfile::Print;
    assert_same_pixels(
        cached_render(&renderer, &tree, 1.0),
        cached_render(&uncached, &tree, 1.0),
    );
    assert!(renderer.cache_stats().pictures.misses > misses);
    misses = renderer.cache_stats().pictures.misses;
    tree.output_options.show_paragraph_marks = true;
    assert_same_pixels(
        cached_render(&renderer, &tree, 1.0),
        cached_render(&uncached, &tree, 1.0),
    );
    assert!(renderer.cache_stats().pictures.misses > misses);
    misses = renderer.cache_stats().pictures.misses;
    let LayerNodeKind::Group { children, .. } = &mut tree.root.kind else {
        unreachable!()
    };
    let LayerNodeKind::Leaf { ops } = &mut children[0].kind else {
        unreachable!()
    };
    let PaintOp::GlyphRun { run, .. } = &mut ops[1] else {
        unreachable!()
    };
    run.placement.run_to_page.e += 0.000_001;
    assert_same_pixels(
        cached_render(&renderer, &tree, 1.0),
        cached_render(&uncached, &tree, 1.0),
    );
    assert!(
        renderer.cache_stats().pictures.misses > misses,
        "sub-JSON precision changes must invalidate"
    );
}

#[test]
fn native_cache_exact_resource_identity_and_invalid_font_metadata_cannot_reuse_ink() {
    let renderer = SkiaLayerRenderer::new();
    let uncached = SkiaLayerRenderer::new().with_cache_limits(0, 0);
    let tree = glyph_tree(TTF, 0, 'A');
    assert_same_pixels(
        cached_render(&renderer, &tree, 1.0),
        cached_render(&uncached, &tree, 1.0),
    );
    cached_render(&renderer, &tree, 1.0);
    assert!(renderer.cache_stats().glyphs.hits > 0);
    let mut invalid = tree.clone();
    invalid.resources.font_resources_mut().faces[0].face_index = 99;
    assert_same_pixels(
        cached_render(&renderer, &invalid, 1.0),
        render(&fallback_tree()),
    );
    let png = |rgba| {
        let mut encoded = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba(rgba))
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        encoded.into_inner()
    };
    let blue = static_tree(bitmap_tree(&png([0, 0, 255, 255])));
    let red = static_tree(bitmap_tree(&png([255, 0, 0, 255])));
    // Both fresh arenas use image ref 0. The bytes, not the numeric ref or a
    // previous arena's successful strict proof, must determine replay identity.
    let blue_pixels = cached_render(&renderer, &blue, 1.0);
    let red_pixels = cached_render(&renderer, &red, 1.0);
    assert!(blue_pixels != red_pixels);
    assert_same_pixels(red_pixels, cached_render(&uncached, &red, 1.0));
    let before = renderer.cache_stats().pictures.misses;
    let next_epoch = red.clone().with_bin_data_epoch(1);
    cached_render(&renderer, &next_epoch, 1.0);
    assert!(renderer.cache_stats().pictures.misses > before);
}

fn image_cache_tree(bytes: Vec<u8>, effect: rhwp::model::image::ImageEffect) -> PageLayerTree {
    let mut image = rhwp::renderer::render_tree::ImageNode::new(1, Some(bytes));
    image.effect = effect;
    PageLayerTree::new(
        80.0,
        60.0,
        LayerNode::leaf(
            BoundingBox::new(0.0, 0.0, 80.0, 60.0),
            None,
            vec![PaintOp::image(
                BoundingBox::new(10.0, 10.0, 30.0, 30.0),
                image,
                None,
            )],
        ),
    )
}

#[test]
fn native_cache_images_svg_and_effects_preserve_resource_only_and_size_dependent_identity() {
    use rhwp::model::image::ImageEffect;
    let renderer = SkiaLayerRenderer::new();
    let uncached = SkiaLayerRenderer::new().with_cache_limits(0, 0);
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"><path fill=\"#0000ff\" d=\"M1 1L9 3L7 9Z\"/></svg>".to_vec();
    let mut tree = image_cache_tree(svg, ImageEffect::RealPic);
    assert_same_pixels(
        cached_render(&renderer, &tree, 1.0),
        cached_render(&uncached, &tree, 1.0),
    );
    cached_render(&renderer, &tree, 1.0);
    assert!(
        renderer.cache_stats().images.hits >= 2,
        "decoded image and effect hit"
    );
    for effect in [ImageEffect::GrayScale, ImageEffect::BlackWhite] {
        let LayerNodeKind::Leaf { ops } = &mut tree.root.kind else {
            unreachable!()
        };
        let PaintOp::Image { image, .. } = &mut ops[0] else {
            unreachable!()
        };
        image.effect = effect;
        assert_same_pixels(
            cached_render(&renderer, &tree, 1.0),
            cached_render(&uncached, &tree, 1.0),
        );
    }
    let before = renderer.cache_stats().images.misses;
    assert_same_pixels(
        cached_render(&renderer, &tree, 2.0),
        cached_render(&uncached, &tree, 2.0),
    );
    assert!(
        renderer.cache_stats().images.misses > before,
        "embedded SVG resolution changes"
    );
    let tree = static_tree(tree);
    for scale in [1.0, 1.5, 2.0] {
        assert_same_pixels(
            cached_render(&renderer, &tree, scale),
            cached_render(&uncached, &tree, scale),
        );
        assert_same_pixels(
            cached_render(&renderer, &tree, scale),
            cached_render(&uncached, &tree, scale),
        );
    }
    let raw_tree = PageLayerTree::new(
        80.0,
        60.0,
        LayerNode::leaf(
            BoundingBox::new(0.0, 0.0, 80.0, 60.0),
            None,
            vec![PaintOp::raw_svg(
                BoundingBox::new(10.0, 10.0, 30.0, 30.0),
                rhwp::renderer::render_tree::RawSvgNode::new(
                    "<path fill=\"#0000ff\" d=\"M10 10H40V40H10Z\"/>".into(),
                ),
            )],
        ),
    );
    assert_same_pixels(
        cached_render(&renderer, &raw_tree, 1.0),
        cached_render(&uncached, &raw_tree, 1.0),
    );
    let before = renderer.cache_stats().images.hits;
    cached_render(&renderer, &raw_tree, 1.0);
    assert!(renderer.cache_stats().images.hits > before);
}

#[test]
fn native_cache_eviction_oversize_clear_and_font_inventory_refresh_are_bounded() {
    let tree = static_tree(glyph_tree(TTF, 0, 'A'));
    let mut other = glyph_tree(TTF, 0, 'A');
    glyph_mut(&mut other)
        .shape_key
        .font_instance
        .synthetic_italic = true;
    let other = static_tree(other);
    let renderer = SkiaLayerRenderer::new().with_cache_limits(1, 1024 * 1024);
    for candidate in [&tree, &other, &tree] {
        cached_render(&renderer, candidate, 1.0);
        for usage in [
            renderer.cache_stats().pictures,
            renderer.cache_stats().glyphs,
            renderer.cache_stats().images,
        ] {
            assert!(usage.entries <= 1 && usage.approximate_bytes <= 1024 * 1024);
        }
    }
    assert!(renderer.cache_stats().pictures.evictions >= 2);
    assert!(renderer.cache_stats().glyphs.evictions >= 2);
    renderer.clear_caches();
    assert_eq!(renderer.cache_stats(), Default::default());
    cached_render(&renderer, &tree, 1.0);
    let renderer = renderer.with_font_paths(&[]);
    assert_eq!(renderer.cache_stats(), Default::default());
    let tiny = SkiaLayerRenderer::new().with_cache_limits(8, 1);
    assert_same_pixels(
        cached_render(&tiny, &tree, 1.0),
        cached_render(&renderer, &tree, 1.0),
    );
    assert_eq!(tiny.cache_stats().pictures.entries, 0);
    assert_eq!(tiny.cache_stats().glyphs.entries, 0);
    assert!(tiny.cache_stats().pictures.skipped_oversized > 0);
    assert!(tiny.cache_stats().glyphs.skipped_oversized > 0);
}

#[test]
fn native_cache_inherited_replay_hints_change_sampling_without_stale_pictures() {
    use rhwp::model::image::ImageEffect;
    let mut encoded = Cursor::new(Vec::new());
    image::RgbaImage::from_fn(2, 2, |x, _| {
        let value = if x == 0 { 0 } else { 255 };
        image::Rgba([value, value, value, 255])
    })
    .write_to(&mut encoded, image::ImageFormat::Png)
    .unwrap();
    let base = static_tree(image_cache_tree(encoded.into_inner(), ImageEffect::RealPic));
    let renderer = SkiaLayerRenderer::new();
    let screen = cached_render(&renderer, &base, 1.0);
    let grays = |image: &image::RgbaImage| {
        image
            .enumerate_pixels()
            .filter(|(x, y, pixel)| {
                (11..39).contains(x) && (11..39).contains(y) && pixel[0] > 0 && pixel[0] < 255
            })
            .count()
    };
    assert!(grays(&screen) > 0);
    let mut nearest = base.clone();
    nearest.root = LayerNode::group(
        nearest.root.bounds,
        None,
        vec![nearest.root],
        CacheHint::PreferRaster,
        GroupKind::Generic,
    );
    let before = renderer.cache_stats().pictures.misses;
    let nearest_pixels = cached_render(&renderer, &nearest, 1.0);
    assert_eq!(grays(&nearest_pixels), 0);
    assert!(
        renderer.cache_stats().pictures.misses > before,
        "ancestor hint enters the static key"
    );
    assert_same_pixels(cached_render(&renderer, &nearest, 1.0), nearest_pixels);
    assert_same_pixels(cached_render(&renderer, &base, 1.0), screen);
    let mut fast = base;
    fast.profile = RenderProfile::FastPreview;
    assert_eq!(grays(&cached_render(&renderer, &fast, 1.0)), 0);
}

#[test]
fn native_cache_picture_budget_includes_oversized_font_svg_viewport() {
    let mut tree = svg_tree("<svg><path fill=\"#0000ff\" d=\"M10 20H40V50H10Z\"/></svg>");
    outline_mut(&mut tree)
        .svg_glyph
        .as_mut()
        .unwrap()
        .font_em_box = Some(BoundingBox::new(10.0, 10.0, 512.0, 512.0));
    let tree = static_tree(tree);
    let renderer = SkiaLayerRenderer::new().with_cache_limits(16, 1024 * 1024);
    cached_render(&renderer, &tree, 1.0);
    let stats = renderer.cache_stats();
    assert_eq!(stats.glyphs.entries, 0);
    assert_eq!(stats.pictures.entries, 0);
    assert!(stats.glyphs.skipped_oversized > 0 && stats.pictures.skipped_oversized > 0);

    let mut unrepresentable = svg_tree("<svg><path d=\"M10 20H40V50H10Z\"/></svg>");
    outline_mut(&mut unrepresentable)
        .svg_glyph
        .as_mut()
        .unwrap()
        .font_em_box = Some(BoundingBox::new(10.0, 10.0, f64::MAX, 1.0));
    let unlimited = SkiaLayerRenderer::new().with_cache_limits(8, usize::MAX);
    assert_same_pixels(
        cached_render(&unlimited, &static_tree(unrepresentable), 1.0),
        render(&fallback_tree()),
    );
    // Even a caller-selected usize::MAX limit must evict a saturated estimate
    // before adding a second entry rather than overflow accounting arithmetic.
    cached_render(&unlimited, &static_tree(glyph_tree(TTF, 0, 'A')), 1.0);
    assert!(unlimited.cache_stats().pictures.evictions > 0);
}

#[test]
fn native_diagnostics_report_per_call_timing_work_and_cached_selection() {
    let renderer = SkiaLayerRenderer::new();
    let tree = static_tree(glyph_tree(TTF, 0, 'A'));
    let first = renderer
        .render_raster_with_options(&tree, Default::default())
        .unwrap();
    let second = renderer
        .render_raster_with_options(&tree, Default::default())
        .unwrap();
    assert_eq!(first.bytes, second.bytes);
    let first = first.diagnostics;
    let second = second.diagnostics;
    assert_eq!(
        first.backend,
        Some(rhwp::renderer::layer_renderer::VariantSelectionBackend::NativeSkia)
    );
    assert_eq!(first.render_profile.as_deref(), Some("screen"));
    for diagnostics in [&first, &second] {
        assert!(diagnostics.raster_setup_time_ns > 0);
        assert!(diagnostics.raster_replay_time_ns > 0);
        assert!(diagnostics.raster_encode_time_ns > 0);
        assert!(
            diagnostics.raster_total_time_ns
                >= diagnostics.raster_setup_time_ns
                    + diagnostics.raster_replay_time_ns
                    + diagnostics.raster_encode_time_ns
        );
        assert!(diagnostics.shaped_measurements.is_empty());
    }
    assert!(first.static_picture_cache_misses > 0);
    assert!(first.static_picture_cache_recordings > 0);
    assert!(first.paint_ops_replayed > 0);
    assert!(second.static_picture_cache_hits > 0);
    assert_eq!(second.static_picture_cache_recordings, 0);
    assert_eq!(second.paint_ops_replayed, 0);
    assert!(second.layer_nodes_replayed < first.layer_nodes_replayed);
    assert_eq!(first.variant_selections, second.variant_selections);
    assert_eq!(
        first.variant_selections[0].selected_variant_kind,
        Some(TextVariantKind::GlyphRun)
    );
    assert!(second.static_picture_cache_approx_bytes > 0);
}

#[test]
fn native_diagnostics_preserve_many_group_order_across_picture_hits_and_new_pages() {
    let mut base = fallback_tree();
    let PaintOp::TextRun { run, .. } = ops_mut(&mut base).remove(0) else {
        unreachable!()
    };
    let bounds = BoundingBox::new(0.0, 0.0, 180.0, 80.0);
    let ops = (0..256)
        .map(|_| PaintOp::text_run(bounds, (*run).clone()))
        .collect();
    let tree = static_tree(PageLayerTree::new(
        180.0,
        80.0,
        LayerNode::leaf(bounds, None, ops),
    ));
    let renderer = SkiaLayerRenderer::new();
    let cold = renderer
        .render_raster_with_options(&tree, Default::default())
        .unwrap();
    let warm = renderer
        .render_raster_with_options(&tree, Default::default())
        .unwrap();
    let expected = (0..256).map(|id| format!("text-{id}")).collect::<Vec<_>>();
    let groups = |output: &rhwp::renderer::layer_renderer::RasterRenderOutput| {
        output
            .diagnostics
            .variant_selections
            .iter()
            .map(|report| report.equivalence_group.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(groups(&cold), expected);
    assert_eq!(
        cold.diagnostics.variant_selections,
        warm.diagnostics.variant_selections
    );
    assert_eq!(cold.bytes, warm.bytes);
    assert!(warm.diagnostics.static_picture_cache_hits > 0);
    assert_eq!(warm.diagnostics.paint_ops_replayed, 0);

    // Retain canonical source bindings but change first-observed order. A new
    // render must rebuild its index, not reuse stale positions or HashMap order.
    let mut reversed = tree.clone();
    let LayerNodeKind::Group { children, .. } = &mut reversed.root.kind else {
        unreachable!()
    };
    let LayerNodeKind::Leaf { ops } = &mut children[0].kind else {
        unreachable!()
    };
    ops.reverse();
    let changed = renderer
        .render_raster_with_options(&reversed, Default::default())
        .unwrap();
    assert_eq!(
        groups(&changed),
        expected.iter().rev().cloned().collect::<Vec<_>>()
    );
    assert!(changed.diagnostics.static_picture_cache_misses > 0);
    let restored = renderer
        .render_raster_with_options(&tree, Default::default())
        .unwrap();
    assert_eq!(groups(&restored), expected);
    assert_eq!(
        cold.diagnostics.variant_selections,
        restored.diagnostics.variant_selections
    );
    assert!(restored.diagnostics.static_picture_cache_hits > 0);
}

#[test]
fn native_diagnostics_match_exact_rejected_and_plain_sources_in_one_leaf() {
    let mut template = glyph_tree(TTF, 0, 'A');
    let template_ops = ops_mut(&mut template).clone();
    let mut ops = Vec::new();
    for id in 0..96_u32 {
        let mut fallback = template_ops[0].clone();
        let PaintOp::TextRun {
            source: Some(source),
            ..
        } = &mut fallback
        else {
            unreachable!()
        };
        source.id = TextSourceId(id);
        ops.push(fallback);
        if id % 3 == 2 {
            continue;
        }
        let mut glyph = template_ops[1].clone();
        let PaintOp::GlyphRun { run, .. } = &mut glyph else {
            unreachable!()
        };
        run.source.id = TextSourceId(id);
        run.variant.equivalence_group = format!("text-{id}");
        run.variant.anchor_op_id = Some(run.variant.equivalence_group.clone());
        if id % 3 == 1 {
            run.glyph_ids[0] = u32::from(u16::MAX);
        }
        let mut second_rejection = (id % 3 == 1).then(|| glyph.clone());
        ops.push(glyph);
        if let Some(PaintOp::GlyphRun { run, .. }) = &mut second_rejection {
            run.variant.variant_id = "second-invalid-glyph-run".into();
        }
        ops.extend(second_rejection);
    }
    let mut tree = PageLayerTree::new(
        180.0,
        80.0,
        LayerNode::leaf(template.root.bounds, None, ops),
    );
    tree.resources = template.resources;
    let tree = static_tree(tree);
    let renderer = SkiaLayerRenderer::new();
    let cold = renderer
        .render_raster_with_options(&tree, Default::default())
        .unwrap();
    let warm = renderer
        .render_raster_with_options(&tree, Default::default())
        .unwrap();
    assert_eq!(cold.bytes, warm.bytes);
    assert_eq!(
        cold.diagnostics.variant_selections,
        warm.diagnostics.variant_selections
    );
    assert!(warm.diagnostics.static_picture_cache_hits > 0);
    let reports = &warm.diagnostics.variant_selections;
    assert_eq!(reports.len(), 96);
    // Sidecar choices are observed during preparation; plain fallbacks follow
    // during replay. Membership indexing must preserve this established order.
    let expected = (0..96_u32)
        .filter(|id| id % 3 != 2)
        .chain((0..96_u32).filter(|id| id % 3 == 2));
    for (report, id) in reports.iter().zip(expected) {
        assert_eq!(report.equivalence_group, format!("text-{id}"));
        let exact = id % 3 == 0;
        assert_eq!(
            report.selected_variant_kind,
            Some(if exact {
                TextVariantKind::GlyphRun
            } else {
                TextVariantKind::TextRun
            })
        );
        assert_eq!(report.fallback_required, !exact);
        assert_eq!(report.rejected_variants.len(), 2 * usize::from(id % 3 == 1));
    }
}

#[test]
fn native_diagnostics_keep_actual_resource_failure_selection_on_picture_hits() {
    let renderer = SkiaLayerRenderer::new();
    let tree = static_tree(svg_tree("<path d=\"M 1 1 L\"/>"));
    let first = renderer
        .render_raster_with_options(&tree, Default::default())
        .unwrap();
    let second = renderer
        .render_raster_with_options(&tree, Default::default())
        .unwrap();
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(
        first.diagnostics.variant_selections,
        second.diagnostics.variant_selections
    );
    let report = &second.diagnostics.variant_selections[0];
    assert_eq!(report.selected_variant_kind, Some(TextVariantKind::TextRun));
    assert!(report.fallback_required);
    assert!(!report.rejected_variants.is_empty());
    assert!(second.diagnostics.static_picture_cache_hits > 0);
}

#[test]
fn native_diagnostics_distinguish_binary_effect_cache_events_from_other_images() {
    use rhwp::model::image::ImageEffect;
    let encode = |width, gray| {
        let mut png = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(width, 1, image::Rgba([gray, gray, gray, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        png.into_inner()
    };
    let renderer = SkiaLayerRenderer::new();
    let bytes = encode(8, 126);
    let tree = image_cache_tree(bytes.clone(), ImageEffect::Pattern8x8);
    let first = renderer
        .render_raster_with_options(&tree, Default::default())
        .unwrap()
        .diagnostics;
    let second = renderer
        .render_raster_with_options(&tree, Default::default())
        .unwrap()
        .diagnostics;
    assert_eq!(first.image_effect_cache_misses, 1);
    assert_eq!(first.image_effect_cache_hits, 0);
    assert_eq!(first.image_effect_preprocessed_bytes, 32);
    assert_eq!(first.image_effect_preprocess_failures, 0);
    assert_eq!(second.image_effect_cache_hits, 1);
    assert_eq!(second.image_effect_cache_misses, 0);
    assert_eq!(second.image_effect_preprocessed_bytes, 0);
    assert!(second.image_effect_cache_approx_bytes >= 32);
    let plain = image_cache_tree(bytes, ImageEffect::RealPic);
    let plain = renderer
        .render_raster_with_options(&plain, Default::default())
        .unwrap()
        .diagnostics;
    assert_eq!(plain.image_effect_cache_hits, 0);
    assert_eq!(plain.image_effect_cache_misses, 0);
    let tiny = SkiaLayerRenderer::new().with_cache_limits(2, 1);
    let tiny = tiny
        .render_raster_with_options(&tree, Default::default())
        .unwrap()
        .diagnostics;
    assert_eq!(tiny.image_effect_cache_skipped_oversized, 1);
    assert_eq!(tiny.image_effect_cache_approx_bytes, 0);
    let limited = SkiaLayerRenderer::new().with_cache_limits(2, 64 * 1024);
    limited
        .render_raster_with_options(&tree, Default::default())
        .unwrap();
    let changed = image_cache_tree(encode(8, 200), ImageEffect::Pattern8x8);
    let evicted = limited
        .render_raster_with_options(&changed, Default::default())
        .unwrap()
        .diagnostics;
    assert_eq!(evicted.image_effect_cache_evictions, 1);
    // The decoded image remains supported, but binary preprocessing has a
    // stricter 4096-pixel per-dimension budget. Its filter fallback is observed.
    let large = image_cache_tree(encode(4097, 100), ImageEffect::BlackWhite);
    let failed = renderer
        .render_raster_with_options(&large, Default::default())
        .unwrap()
        .diagnostics;
    assert_eq!(failed.image_effect_preprocess_failures, 1);
    assert_eq!(failed.image_effect_fallback_to_filter, 1);
    let negative_hit = renderer
        .render_raster_with_options(&large, Default::default())
        .unwrap()
        .diagnostics;
    assert_eq!(negative_hit.image_effect_cache_hits, 1);
    assert_eq!(negative_hit.image_effect_preprocess_failures, 0);
    assert_eq!(negative_hit.image_effect_fallback_to_filter, 1);
}

#[test]
fn native_diagnostics_report_bounded_manual_tile_fallback() {
    use rhwp::model::style::ImageFillMode;
    let mut png = Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(2, 1, image::Rgba([0, 0, 0, 255]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let mut tree = image_cache_tree(png.into_inner(), rhwp::model::image::ImageEffect::RealPic);
    let PaintOp::Image { bbox, image, .. } = &mut ops_mut(&mut tree)[0] else {
        unreachable!()
    };
    *bbox = BoundingBox::new(0.0, 0.0, 5000.0, 1.0);
    image.fill_mode = Some(ImageFillMode::TileAll);
    // Dividing the smallest positive f32 tile width by a two-pixel source
    // underflows the shader matrix. This exercises the real fallback without
    // a source-side test hook or an unbounded surface allocation.
    image.original_size = Some((f64::from(f32::from_bits(1)), 1.0));
    let output = SkiaLayerRenderer::new()
        .render_raster_with_options(&tree, Default::default())
        .unwrap();
    assert_eq!(output.diagnostics.tile_fallback_cap_hits, 1);
    assert_eq!((output.width, output.height), (80, 60));
}

#[test]
fn native_picture_cull_retains_ink_outside_logical_node_bounds() {
    let mut tree = glyph_tree(TTF, 0, 'A');
    glyph_mut(&mut tree).paint_style.shadow_type = 1;
    glyph_mut(&mut tree).paint_style.shadow_offset_x = -8.0;
    glyph_mut(&mut tree).paint_style.shadow_offset_y = -6.0;
    ops_mut(&mut tree).push(PaintOp::rectangle(
        BoundingBox::new(20.0, 20.0, 20.0, 20.0),
        rhwp::renderer::render_tree::RectangleNode::new(
            0.0,
            rhwp::renderer::ShapeStyle {
                stroke_color: Some(0x0000ff),
                stroke_width: 20.0,
                ..Default::default()
            },
            None,
        ),
    ));
    tree.root.bounds = BoundingBox::new(24.0, 24.0, 1.0, 1.0);
    let tree = static_tree(tree);
    let direct = SkiaLayerRenderer::new().with_cache_limits(0, 0);
    let renderer = SkiaLayerRenderer::new();
    let expected = cached_render(&direct, &tree, 1.0);
    assert!(expected.get_pixel(12, 25)[3] > 200);
    assert_same_pixels(cached_render(&renderer, &tree, 1.0), expected.clone());
    assert_same_pixels(cached_render(&renderer, &tree, 1.0), expected);
    assert!(renderer.cache_stats().pictures.hits > 0);
}
