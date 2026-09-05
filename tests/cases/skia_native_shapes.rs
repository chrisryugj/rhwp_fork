//! Native shapes retain committed Skia paint capabilities on the current tree.

#![cfg(all(feature = "native-skia", not(target_arch = "wasm32")))]

use rhwp::paint::{LayerNode, PageLayerTree, PaintOp};
use rhwp::renderer::layer_renderer::LayerRasterRenderer;
use rhwp::renderer::render_tree::{BoundingBox, PageBackgroundNode, RectangleNode};
use rhwp::renderer::skia::SkiaLayerRenderer;
use rhwp::renderer::{GradientFillInfo, PatternFillInfo, ShapeStyle};

fn tree(op: PaintOp) -> PageLayerTree {
    PageLayerTree::new(
        80.0,
        60.0,
        LayerNode::leaf(BoundingBox::new(0.0, 0.0, 80.0, 60.0), None, vec![op]),
    )
}

fn pixels(tree: &PageLayerTree) -> image::RgbaImage {
    let bytes = SkiaLayerRenderer::new().render_png(tree).unwrap();
    image::load_from_memory(&bytes).unwrap().to_rgba8()
}

fn gradient(kind: i16) -> GradientFillInfo {
    GradientFillInfo {
        gradient_type: kind,
        angle: 90,
        center_x: 50,
        center_y: 50,
        colors: vec![0xff0000, 0x0000ff],
        positions: vec![0.0, 1.0],
    }
}

#[test]
fn native_shapes_render_linear_and_radial_gradient_colors_and_opacity() {
    for kind in [1, 2] {
        let image = pixels(&tree(PaintOp::rectangle(
            BoundingBox::new(10.0, 10.0, 60.0, 30.0),
            RectangleNode::new(
                0.0,
                ShapeStyle {
                    opacity: 0.5,
                    ..Default::default()
                },
                Some(Box::new(gradient(kind))),
            ),
        )));
        let (blue, red) = if kind == 1 {
            ((12, 25), (67, 25))
        } else {
            ((40, 25), (12, 25))
        };
        let blue = image.get_pixel(blue.0, blue.1);
        let red = image.get_pixel(red.0, red.1);
        assert!(blue[2] > 220 && red[0] > 220);
        assert!((126..=129).contains(&blue[3]));
    }
    let image = pixels(&tree(PaintOp::page_background(
        BoundingBox::new(10.0, 10.0, 60.0, 30.0),
        PageBackgroundNode {
            background_color: None,
            border_color: None,
            border_width: 0.0,
            gradient: Some(Box::new(gradient(1))),
            image: None,
        },
    )));
    assert!(image.get_pixel(12, 25)[2] > 220 && image.get_pixel(67, 25)[0] > 220);
}

#[test]
fn native_shapes_render_each_hatch_pattern_instead_of_only_background() {
    for pattern_type in 0..6 {
        let image = pixels(&tree(PaintOp::rectangle(
            BoundingBox::new(10.0, 10.0, 60.0, 30.0),
            RectangleNode::new(
                0.0,
                ShapeStyle {
                    pattern: Some(PatternFillInfo {
                        pattern_type,
                        pattern_color: 0x0000ff,
                        background_color: 0x00ff00,
                    }),
                    ..Default::default()
                },
                None,
            ),
        )));
        assert!(
            image.pixels().any(|pixel| pixel[0] > 100 && pixel[1] < 200),
            "pattern {pattern_type} foreground"
        );
        assert!(
            image.pixels().any(|pixel| pixel[1] > 220 && pixel[0] < 30),
            "pattern {pattern_type} background"
        );
    }
}

#[test]
fn native_shapes_render_each_line_head_and_parallel_line_variant() {
    use rhwp::renderer::render_tree::LineNode;
    use rhwp::renderer::{ArrowStyle, LineRenderType, LineStyle};
    let line = |style| {
        tree(PaintOp::line(
            BoundingBox::new(10.0, 20.0, 55.0, 20.0),
            LineNode::new(10.0, 30.0, 65.0, 30.0, style),
        ))
    };
    for arrow in [
        ArrowStyle::Arrow,
        ArrowStyle::ConcaveArrow,
        ArrowStyle::Diamond,
        ArrowStyle::OpenDiamond,
        ArrowStyle::Circle,
        ArrowStyle::OpenCircle,
        ArrowStyle::Square,
        ArrowStyle::OpenSquare,
    ] {
        let image = pixels(&line(LineStyle {
            width: 2.0,
            start_arrow: arrow,
            end_arrow: arrow,
            start_arrow_size: 8,
            end_arrow_size: 8,
            ..Default::default()
        }));
        assert!(
            image
                .enumerate_pixels()
                .any(|(_, y, pixel)| !(29..=30).contains(&y) && pixel[3] > 64),
            "{arrow:?}"
        );
    }
    for line_type in [
        LineRenderType::Double,
        LineRenderType::ThickThinDouble,
        LineRenderType::ThinThickDouble,
        LineRenderType::ThinThickThinTriple,
    ] {
        let image = pixels(&line(LineStyle {
            width: 8.0,
            line_type,
            ..Default::default()
        }));
        let segments = (20..40)
            .fold((0, false), |(count, previous), y| {
                let ink = image.get_pixel(40, y)[3] > 100;
                (count + usize::from(ink && !previous), ink)
            })
            .0;
        assert_eq!(
            segments,
            if line_type == LineRenderType::ThinThickThinTriple {
                3
            } else {
                2
            }
        );
    }
}

#[test]
fn native_shapes_replay_connector_heads_along_curve_tangents() {
    use rhwp::renderer::render_tree::PathNode;
    use rhwp::renderer::{ArrowStyle, LineStyle, PathCommand};
    let mut path = PathNode::new(
        vec![
            PathCommand::MoveTo(10.0, 45.0),
            PathCommand::CurveTo(15.0, 45.0, 65.0, 45.0, 65.0, 15.0),
        ],
        ShapeStyle {
            stroke_color: Some(0),
            stroke_width: 2.0,
            ..Default::default()
        },
        None,
    );
    let bbox = BoundingBox::new(5.0, 10.0, 70.0, 40.0);
    let plain = pixels(&tree(PaintOp::path(bbox, path.clone())));
    path.connector_endpoints = Some((10.0, 45.0, 65.0, 15.0));
    path.line_style = Some(LineStyle {
        width: 2.0,
        start_arrow: ArrowStyle::Arrow,
        end_arrow: ArrowStyle::Arrow,
        start_arrow_size: 8,
        end_arrow_size: 8,
        ..Default::default()
    });
    let image = pixels(&tree(PaintOp::path(bbox, path)));
    assert!(plain != image);
    assert!(image.get_pixel(62, 26)[3] > plain.get_pixel(62, 26)[3]);
}

#[test]
fn native_shapes_fast_raster_hint_disables_only_its_subtree_antialiasing() {
    use rhwp::paint::{CacheHint, GroupKind, RenderProfile};
    use rhwp::renderer::render_tree::PathNode;
    use rhwp::renderer::PathCommand;
    let mut tree = tree(PaintOp::path(
        BoundingBox::new(10.0, 10.0, 60.0, 30.0),
        PathNode::new(
            vec![
                PathCommand::MoveTo(10.25, 10.25),
                PathCommand::LineTo(70.25, 30.25),
                PathCommand::LineTo(10.25, 40.25),
                PathCommand::ClosePath,
            ],
            ShapeStyle {
                fill_color: Some(0),
                ..Default::default()
            },
            None,
        ),
    ));
    tree.profile = RenderProfile::FastPreview;
    let smooth = pixels(&tree);
    assert!(smooth.pixels().any(|pixel| pixel[3] > 0 && pixel[3] < 255));
    tree.root = LayerNode::group(
        tree.root.bounds,
        None,
        vec![tree.root],
        CacheHint::PreferRaster,
        GroupKind::Generic,
    );
    let fast = pixels(&tree);
    assert!(!fast.pixels().any(|pixel| pixel[3] > 0 && pixel[3] < 255));
    tree.profile = RenderProfile::Screen;
    assert_eq!(pixels(&tree).as_raw(), smooth.as_raw());
}

#[test]
fn native_images_binary_effects_use_source_thresholds_and_preserve_alpha() {
    use rhwp::model::image::ImageEffect;
    use rhwp::renderer::render_tree::ImageNode;
    let image_tree = |image: image::RgbaImage, effect| {
        let (width, height) = image.dimensions();
        let mut encoded = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let mut node = ImageNode::new(1, Some(encoded.into_inner()));
        node.effect = effect;
        tree(PaintOp::image(
            BoundingBox::new(10.0, 10.0, f64::from(width), f64::from(height)),
            node,
            None,
        ))
    };
    let input = image::RgbaImage::from_fn(2, 1, |x, _| {
        if x == 0 {
            image::Rgba([127, 127, 127, 37])
        } else {
            image::Rgba([128, 128, 128, 191])
        }
    });
    let output = pixels(&image_tree(input, ImageEffect::BlackWhite));
    assert_eq!(output.get_pixel(10, 10).0, [0, 0, 0, 37]);
    assert_eq!(output.get_pixel(11, 10).0, [255, 255, 255, 191]);
    let input = image::RgbaImage::from_pixel(8, 8, image::Rgba([126, 126, 126, 173]));
    let tree = image_tree(input, ImageEffect::Pattern8x8);
    let renderer = SkiaLayerRenderer::new();
    for _ in 0..2 {
        let bytes = renderer.render_png(&tree).unwrap();
        let output = image::load_from_memory(&bytes).unwrap().to_rgba8();
        for y in 0..8 {
            for x in 0..8 {
                let value = if (x + y) % 2 == 0 { 255 } else { 0 };
                assert_eq!(
                    output.get_pixel(x + 10, y + 10).0,
                    [value, value, value, 173],
                    "({x},{y})"
                );
            }
        }
    }
    assert!(renderer.cache_stats().images.hits >= 2);
}

#[test]
fn native_images_binary_effects_preserve_nearest_phase_when_scaled_and_cached() {
    use rhwp::model::image::ImageEffect;
    use rhwp::renderer::render_tree::ImageNode;
    let renderer = SkiaLayerRenderer::new();
    for effect in [ImageEffect::BlackWhite, ImageEffect::Pattern8x8] {
        let source = if effect == ImageEffect::BlackWhite {
            image::RgbaImage::from_fn(8, 8, |x, _| image::Rgba([if x < 4 { 0 } else { 255 }; 4]))
        } else {
            image::RgbaImage::from_pixel(8, 8, image::Rgba([126, 126, 126, 173]))
        };
        let source = image::RgbaImage::from_fn(8, 8, |x, y| {
            let mut pixel = *source.get_pixel(x, y);
            pixel[3] = 173;
            pixel
        });
        let mut encoded = std::io::Cursor::new(Vec::new());
        source
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let mut node = ImageNode::new(1, Some(encoded.into_inner()));
        node.effect = effect;
        let mut tree = tree(PaintOp::image(
            BoundingBox::new(10.0, 10.0, 32.0, 32.0),
            node,
            None,
        ));
        tree.profile = rhwp::paint::RenderProfile::HighQuality;
        for _ in 0..2 {
            let png = renderer.render_png(&tree).unwrap();
            let output = image::load_from_memory(&png).unwrap().to_rgba8();
            for y in 0..32 {
                for x in 0..32 {
                    let white = if effect == ImageEffect::BlackWhite {
                        x >= 16
                    } else {
                        (x / 4 + y / 4) % 2 == 0
                    };
                    let value = if white { 255 } else { 0 };
                    assert_eq!(
                        output.get_pixel(x + 10, y + 10).0,
                        [value, value, value, 173]
                    );
                }
            }
        }
    }
    assert!(renderer.cache_stats().images.hits >= 2);
}
