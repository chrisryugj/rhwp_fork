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
