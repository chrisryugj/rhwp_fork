//! Native fills retained from the committed Skia branch.

use super::renderer::colorref_to_skia;
use crate::renderer::{GradientFillInfo, PatternFillInfo, ShapeStyle};
use skia_safe::{
    gradient::{shaders, Colors, Gradient, Interpolation},
    paint, surfaces, Color4f, FilterMode, MipmapMode, Paint, Point, Rect, SamplingOptions,
    TileMode,
};

pub(super) fn shape_fill(
    bounds: Rect,
    style: &ShapeStyle,
    gradient: Option<&GradientFillInfo>,
    antialias: bool,
) -> Option<Paint> {
    let mut paint = Paint::default();
    paint.set_anti_alias(antialias);
    if let Some(shader) = make_fill_shader(bounds, style.pattern.as_ref(), gradient) {
        paint.set_alpha_f(style.opacity.clamp(0.0, 1.0) as f32);
        paint.set_shader(shader);
    } else {
        // Retain the current renderer's deterministic malformed/single-stop
        // fallback rather than removing a previously visible shape.
        let color = gradient
            .and_then(|gradient| gradient.colors.first().copied())
            .or(style.pattern.map(|pattern| pattern.background_color))
            .or(style.fill_color)?;
        paint.set_color(colorref_to_skia(color, style.opacity as f32));
    }
    Some(paint)
}

fn make_fill_shader(
    bounds: Rect,
    pattern: Option<&PatternFillInfo>,
    gradient: Option<&GradientFillInfo>,
) -> Option<skia_safe::Shader> {
    if let Some(gradient) = gradient {
        if let Some(shader) = make_gradient_shader(gradient, bounds) {
            return Some(shader);
        }
    }
    pattern.and_then(make_pattern_shader)
}

fn make_gradient_shader(info: &GradientFillInfo, bounds: Rect) -> Option<skia_safe::Shader> {
    if info.colors.len() < 2 || info.colors.len() > 4096 || !bounds.is_finite() || bounds.is_empty()
    {
        return None;
    }

    let colors: Vec<Color4f> = info
        .colors
        .iter()
        .copied()
        .map(|color| Color4f::from(colorref_to_skia(color, 1.0)))
        .collect();
    let positions: Vec<f32> = if info.positions.len() == colors.len()
        && info.positions.iter().all(|position| position.is_finite())
    {
        info.positions
            .iter()
            .map(|position| position.clamp(0.0, 1.0) as f32)
            .collect()
    } else {
        Vec::new()
    };
    let colors = if positions.is_empty() {
        Colors::new_evenly_spaced(&colors, TileMode::Clamp, None)
    } else {
        Colors::new(&colors, Some(&positions), TileMode::Clamp, None)
    };
    let shader_gradient = Gradient::new(colors, Interpolation::default());

    if matches!(info.gradient_type, 2..=4) {
        let center = Point::new(
            bounds.left + bounds.width() * (info.center_x as f32 / 100.0),
            bounds.top + bounds.height() * (info.center_y as f32 / 100.0),
        );
        let radius = bounds.width().max(bounds.height()) / 2.0;
        return shaders::radial_gradient((center, radius.max(1.0)), &shader_gradient, None);
    }

    let (start, end) = angle_to_gradient_points(info.angle, bounds);
    shaders::linear_gradient((start, end), &shader_gradient, None)
}

fn angle_to_gradient_points(angle: i16, bounds: Rect) -> (Point, Point) {
    let x = bounds.left;
    let y = bounds.top;
    let width = bounds.width();
    let height = bounds.height();
    let angle = ((angle % 360 + 360) % 360) as f32;

    match angle as i32 {
        0 => (Point::new(x, y), Point::new(x, y + height)),
        45 => (Point::new(x, y), Point::new(x + width, y + height)),
        90 => (Point::new(x, y), Point::new(x + width, y)),
        135 => (Point::new(x, y + height), Point::new(x + width, y)),
        180 => (Point::new(x, y + height), Point::new(x, y)),
        225 => (Point::new(x + width, y + height), Point::new(x, y)),
        270 => (Point::new(x + width, y), Point::new(x, y)),
        315 => (Point::new(x + width, y), Point::new(x, y + height)),
        _ => {
            let radians = angle.to_radians();
            let sin_angle = radians.sin();
            let cos_angle = radians.cos();
            let center_x = x + width / 2.0;
            let center_y = y + height / 2.0;
            (
                Point::new(
                    center_x - sin_angle * width / 2.0,
                    center_y - cos_angle * height / 2.0,
                ),
                Point::new(
                    center_x + sin_angle * width / 2.0,
                    center_y + cos_angle * height / 2.0,
                ),
            )
        }
    }
}

fn make_pattern_shader(pattern: &PatternFillInfo) -> Option<skia_safe::Shader> {
    let mut surface = surfaces::raster_n32_premul((6, 6))?;
    let canvas = surface.canvas();

    let mut background = Paint::default();
    background.set_anti_alias(false);
    background.set_style(paint::Style::Fill);
    background.set_color(colorref_to_skia(pattern.background_color, 1.0));
    canvas.draw_rect(Rect::from_xywh(0.0, 0.0, 6.0, 6.0), &background);

    let mut foreground = Paint::default();
    foreground.set_anti_alias(true);
    foreground.set_style(paint::Style::Stroke);
    foreground.set_stroke_width(1.0);
    foreground.set_color(colorref_to_skia(pattern.pattern_color, 1.0));

    match pattern.pattern_type {
        0 => {
            canvas.draw_line((0.0, 3.0), (6.0, 3.0), &foreground);
        }
        1 => {
            canvas.draw_line((3.0, 0.0), (3.0, 6.0), &foreground);
        }
        2 => {
            canvas.draw_line((6.0, 0.0), (0.0, 6.0), &foreground);
        }
        3 => {
            canvas.draw_line((0.0, 0.0), (6.0, 6.0), &foreground);
        }
        4 => {
            canvas.draw_line((3.0, 0.0), (3.0, 6.0), &foreground);
            canvas.draw_line((0.0, 3.0), (6.0, 3.0), &foreground);
        }
        5 => {
            canvas.draw_line((0.0, 0.0), (6.0, 6.0), &foreground);
            canvas.draw_line((6.0, 0.0), (0.0, 6.0), &foreground);
        }
        _ => {}
    }

    surface.image_snapshot().to_shader(
        Some((TileMode::Repeat, TileMode::Repeat)),
        SamplingOptions::new(FilterMode::Nearest, MipmapMode::None),
        None,
    )
}
