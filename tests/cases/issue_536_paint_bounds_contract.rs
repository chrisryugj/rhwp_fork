//! P47: recording bounds are derived paint metadata, never layout/clip authority.

use rhwp::model::style::UnderlineType;
use rhwp::paint::*;
use rhwp::renderer::layer_renderer::LayerRenderer;
use rhwp::renderer::render_tree::{
    BoundingBox, LineNode, PageRenderTree, PathNode, RectangleNode, RenderNode, RenderNodeType,
    ShapeTransform, TextLineNode, TextRunNode,
};
use rhwp::renderer::svg_layer::SvgLayerRenderer;
use rhwp::renderer::{
    ArrowStyle, LineRenderType, LineStyle, ShadowStyle, ShapeStyle, TabLeaderInfo, TextStyle,
};

fn rect(bounds: BoundingBox, stroke_width: f64) -> PaintOp {
    PaintOp::rectangle(
        bounds,
        RectangleNode::new(
            0.0,
            ShapeStyle {
                fill_color: Some(0x00332211),
                stroke_color: Some(0x00000000),
                stroke_width,
                ..Default::default()
            },
            None,
        ),
    )
}

fn text(text: &str) -> TextRunNode {
    TextRunNode {
        text: text.to_string(),
        style: TextStyle {
            font_size: 16.0,
            font_family: "fixture".to_string(),
            shade_color: 0x00ff_ffff,
            ..Default::default()
        },
        char_shape_id: Some(7),
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
        baseline: 12.0,
        field_marker: Default::default(),
        layout_positions: None,
        display_text: None,
    }
}

fn svg(tree: &PageLayerTree) -> String {
    let mut renderer = SvgLayerRenderer::new();
    renderer.render_page(tree).unwrap();
    renderer.output().to_string()
}

#[test]
fn separates_stroke_arrow_shadow_and_rotated_ink_from_logical_bounds() {
    let logical = BoundingBox::new(10.0, 20.0, 30.0, 10.0);
    let simple = rect(logical, 4.0);
    assert_eq!(simple.bounds(), logical);
    assert_eq!(simple.paint_bounds().logical, logical);
    assert_eq!(
        simple.visual_bounds(),
        BoundingBox::new(8.0, 18.0, 34.0, 14.0)
    );

    let mut rotated = simple.clone();
    let PaintOp::Rectangle { rect, .. } = &mut rotated else {
        unreachable!()
    };
    rect.transform.rotation = 90.0;
    let bounds = rotated.visual_bounds();
    assert!((bounds.width - 14.0).abs() < 1e-9);
    assert!((bounds.height - 34.0).abs() < 1e-9);
    assert_eq!(rotated.bounds(), logical);

    let line_style = LineStyle {
        width: 2.0,
        line_type: LineRenderType::Double,
        end_arrow: ArrowStyle::Arrow,
        shadow: Some(ShadowStyle {
            shadow_type: 1,
            color: 0,
            offset_x: 5.0,
            offset_y: 8.0,
            alpha: 128,
        }),
        ..Default::default()
    };
    let line = PaintOp::line(
        logical,
        LineNode::new(10.0, 25.0, 40.0, 25.0, line_style.clone()),
    );
    assert!(line
        .visual_bounds()
        .contains(&BoundingBox::new(-6.0, 4.0, 62.0, 42.0)));
    assert!(line.visual_bounds().x + line.visual_bounds().width > 56.0);
    let mut path = PathNode::new(Vec::new(), ShapeStyle::default(), None);
    path.line_style = Some(line_style);
    let path = PaintOp::path(logical, path);
    assert!(path.visual_bounds().contains(&logical));
    assert!(path.visual_bounds().width > logical.width + 32.0);
    assert_eq!(path.bounds(), logical);
}

#[test]
fn explicit_text_bounds_use_producer_positions_and_trailing_space_extent() {
    let logical = BoundingBox::new(10.0, 20.0, 1.0, 1.0);
    let mut run = text("A  ");
    run.style.underline = UnderlineType::Bottom;
    run.layout_positions = Some(vec![0.0, 10.0, 40.0, 70.0]);
    let mut underline =
        PaintOp::text_decoration(logical, run.clone(), TextDecorationKind::Underline);
    let untrimmed = underline.visual_bounds();
    let PaintOp::TextDecoration {
        trim_trailing_spaces,
        ..
    } = &mut underline
    else {
        unreachable!()
    };
    *trim_trailing_spaces = 2;
    let trimmed = underline.visual_bounds();
    assert_eq!(untrimmed.x + untrimmed.width, 82.0);
    assert_eq!(trimmed.x + trimmed.width, 22.0);
    assert!(trimmed.y + trimmed.height > logical.y + logical.height);
    assert_eq!(underline.bounds(), logical);

    run.style.tab_leaders = vec![TabLeaderInfo {
        start_x: 20.0,
        end_x: 50.0,
        fill_type: 1,
    }];
    let leader = PaintOp::tab_leader(logical, run.clone());
    assert_eq!(
        leader.visual_bounds().x + leader.visual_bounds().width,
        60.0
    );
    run.is_para_end = true;
    let mark = PaintOp::text_control_mark(logical, run.clone());
    assert!(mark.visual_bounds().width > logical.width);
    assert!(mark.visual_bounds().y < logical.y);
    run.style.shadow_type = 1;
    run.style.shadow_offset_x = 24.0;
    run.style.shadow_offset_y = -10.0;
    let fallback = PaintOp::text_run(logical, run);
    assert!(fallback.visual_bounds().x + fallback.visual_bounds().width >= 59.0);
    assert_eq!(fallback.bounds(), logical);
}

#[test]
fn clip_options_intersect_derived_bounds_without_slop_or_layout_rewrites() {
    let logical = BoundingBox::new(10.0, 20.0, 30.0, 10.0);
    let leaf = LayerNode::leaf(logical, Some(3), vec![rect(logical, 8.0)]);
    let clip = BoundingBox::new(12.0, 22.0, 12.0, 7.0);
    let clipped = LayerNode::clip_rect(logical, Some(2), clip, leaf, ClipKind::Body);
    let root = LayerNode::group(
        logical,
        Some(1),
        vec![clipped],
        CacheHint::Static,
        GroupKind::Header,
    );
    let mut tree = PageLayerTree::new(100.0, 100.0, root);
    assert_eq!(tree.paint_bounds().logical, logical);
    assert_eq!(tree.visual_bounds(), clip);
    let json: serde_json::Value = serde_json::from_str(&tree.to_json()).unwrap();
    assert_eq!(
        json["root"]["bounds"],
        serde_json::to_value(logical).unwrap()
    );
    assert_eq!(
        json["root"]["visualBounds"],
        serde_json::to_value(clip).unwrap()
    );
    assert_eq!(
        json["root"]["children"][0]["clip"],
        serde_json::to_value(clip).unwrap()
    );
    tree.output_options.clip_enabled = false;
    assert_eq!(
        tree.visual_bounds(),
        BoundingBox::new(6.0, 16.0, 38.0, 18.0)
    );
    let unclipped: serde_json::Value = serde_json::from_str(&tree.to_json()).unwrap();
    assert_eq!(
        unclipped["root"]["children"][0]["clip"],
        json["root"]["children"][0]["clip"]
    );
    assert_ne!(
        unclipped["root"]["visualBounds"],
        json["root"]["visualBounds"]
    );
}

#[test]
fn mutated_paint_refreshes_recording_metadata_but_not_debug_or_resource_identity() {
    let logical = BoundingBox::new(10.0, 20.0, 30.0, 10.0);
    let root = LayerNode::group(
        logical,
        Some(1),
        vec![LayerNode::leaf(logical, Some(2), vec![rect(logical, 2.0)])],
        CacheHint::Static,
        GroupKind::TextLine(TextLineNode::with_para(10.0, 8.0, 0, 7)),
    );
    let mut tree = PageLayerTree::new(100.0, 100.0, root).with_output_options(LayerOutputOptions {
        debug_overlay: true,
        ..Default::default()
    });
    let before_svg = svg(&tree);
    let before: serde_json::Value = serde_json::from_str(&tree.to_json()).unwrap();
    let LayerNodeKind::Group { children, .. } = &mut tree.root.kind else {
        unreachable!()
    };
    let LayerNodeKind::Leaf { ops } = &mut children[0].kind else {
        unreachable!()
    };
    let PaintOp::Rectangle { rect, .. } = &mut ops[0] else {
        unreachable!()
    };
    rect.style.stroke_width = 20.0;
    let after: serde_json::Value = serde_json::from_str(&tree.to_json()).unwrap();
    assert_ne!(
        before["root"]["visualBounds"],
        after["root"]["visualBounds"]
    );
    assert_ne!(
        before["root"]["children"][0]["ops"],
        after["root"]["children"][0]["ops"]
    );
    assert_eq!(before["root"]["bounds"], after["root"]["bounds"]);
    assert_eq!(
        before["root"]["children"][0]["bounds"],
        after["root"]["children"][0]["bounds"]
    );
    assert_eq!(
        before["root"]["children"][0]["sourceNodeId"],
        after["root"]["children"][0]["sourceNodeId"]
    );
    assert_eq!(before["resources"], after["resources"]);
    assert_eq!(before["textSources"], after["textSources"]);
    let after_svg = svg(&tree);
    let debug_start = "<g id=\"debug-overlay\"";
    assert!(before_svg.contains("s0:pi=7"));
    assert_eq!(
        &before_svg[before_svg.find(debug_start).unwrap()..],
        &after_svg[after_svg.find(debug_start).unwrap()..]
    );
    let v2: serde_json::Value = serde_json::from_str(&tree.to_json_v2_compat().unwrap()).unwrap();
    assert_eq!(v2["root"]["visualBounds"], after["root"]["visualBounds"]);
    assert_eq!(after["schemaMinorVersion"], 25);
    assert!(after["knownFeatures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|feature| feature == "layer.visualBounds"));
}

#[test]
fn font_svg_and_bitmap_page_placement_is_not_translated_twice() {
    const FONT: &[u8] = include_bytes!("../fixtures/fonts/RHWPBitmapSvgGlyphSmoke.ttf");
    for (glyph, svg_payload) in [("\u{E101}", true), ("\u{E100}", false)] {
        let logical = BoundingBox::new(10.0, 20.0, 16.0, 18.0);
        let mut tree = PageLayerTree::new(
            100.0,
            100.0,
            LayerNode::leaf(logical, None, vec![PaintOp::text_run(logical, text(glyph))]),
        );
        let report = lower_font_native_glyph_sidecars(
            &mut tree.root,
            &mut tree.resources,
            &[EmbeddedFontFace {
                char_shape_id: 7,
                language_index: 0,
                family: "fixture",
                alternate_family: None,
                bytes: FONT,
                face_index: 0,
            }],
        );
        assert_eq!(report.emitted_svg_glyphs + report.emitted_bitmap_glyphs, 1);
        let LayerNodeKind::Leaf { ops } = &mut tree.root.kind else {
            unreachable!()
        };
        let op = ops
            .iter_mut()
            .find(|op| matches!(op, PaintOp::GlyphOutline { .. }))
            .unwrap();
        let PaintOp::GlyphOutline { outline, .. } = op else {
            unreachable!()
        };
        let prior_key = outline.payload_resource_key().unwrap();
        let viewport = BoundingBox::new(10.0, 32.0, 80.0, 80.0);
        if svg_payload {
            outline.svg_glyph.as_mut().unwrap().font_em_box = Some(viewport);
        } else {
            outline.bitmap_glyph.as_mut().unwrap().placement = viewport;
        }
        assert_ne!(prior_key, outline.payload_resource_key().unwrap());
        assert_eq!(outline.placement.run_to_page.e, 10.0);
        assert_eq!(outline.placement.run_to_page.f, 32.0);
        let visual = op.visual_bounds();
        assert!(visual.contains(&viewport));
        assert_eq!(visual.x + visual.width, 90.0);
        assert_eq!(visual.y + visual.height, 112.0);
        assert_eq!(op.bounds(), logical);
    }
}

#[test]
fn builder_preserves_resolved_overflow_clip_and_lowers_translucent_control_once() {
    let logical = BoundingBox::new(20.0, 20.0, 40.0, 40.0);
    let resolved_clip = BoundingBox::new(10.0, 20.0, 80.0, 75.0);
    let object_bounds = BoundingBox::new(70.0, 70.0, 20.0, 25.0);
    let mut render_tree = PageRenderTree::new(0, 100.0, 100.0);
    let mut body = RenderNode::new(
        1,
        RenderNodeType::Body {
            clip_rect: Some(resolved_clip),
        },
        logical,
    );
    body.children.push(RenderNode::new(
        2,
        RenderNodeType::Rectangle(RectangleNode::new(
            0.0,
            ShapeStyle {
                fill_color: Some(0x00332211),
                opacity: 0.5,
                ..Default::default()
            },
            None,
        )),
        object_bounds,
    ));
    render_tree.root.children.push(body);
    let tree = LayerBuilder::new(RenderProfile::Screen).build(&render_tree);
    let value: serde_json::Value = serde_json::from_str(&tree.to_json()).unwrap();
    let body = &value["root"]["children"][0];
    assert_eq!(body["bounds"], serde_json::to_value(logical).unwrap());
    assert_eq!(body["clip"], serde_json::to_value(resolved_clip).unwrap());
    assert_eq!(
        body["visualBounds"],
        serde_json::to_value(object_bounds).unwrap()
    );
    assert_eq!(
        value.to_string().matches("\"type\":\"rectangle\"").count(),
        1
    );
    let output = svg(&tree);
    assert_eq!(output.matches("fill=\"#112233\"").count(), 1);
    assert!(!body.to_string().contains("clipPolicy"));
}

#[test]
fn empty_or_clipped_subtrees_omit_visual_metadata_and_invalid_effects_stay_finite() {
    let logical = BoundingBox::new(10.0, 20.0, 30.0, 10.0);
    let empty = PageLayerTree::new(100.0, 100.0, LayerNode::leaf(logical, None, Vec::new()));
    assert_eq!(empty.visual_bounds(), BoundingBox::default());
    let value: serde_json::Value = serde_json::from_str(&empty.to_json()).unwrap();
    assert!(value["root"].get("visualBounds").is_none());
    let clipped = PageLayerTree::new(
        100.0,
        100.0,
        LayerNode::clip_rect(
            logical,
            None,
            BoundingBox::new(80.0, 80.0, 1.0, 1.0),
            LayerNode::leaf(logical, None, vec![rect(logical, 2.0)]),
            ClipKind::Generic,
        ),
    );
    let value: serde_json::Value = serde_json::from_str(&clipped.to_json()).unwrap();
    assert!(value["root"].get("visualBounds").is_none());
    assert!(value["root"]["child"].get("visualBounds").is_some());
    let mut malformed = rect(logical, f64::INFINITY);
    let PaintOp::Rectangle { rect, .. } = &mut malformed else {
        unreachable!()
    };
    rect.transform = ShapeTransform {
        rotation: f64::NAN,
        ..Default::default()
    };
    assert_eq!(malformed.visual_bounds(), logical);
}
