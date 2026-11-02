// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: chart.rs
// @Software: resource-pulse

use crate::theme;
use gpui::{Div, PathBuilder, canvas, div, point, prelude::*, px, rgb};

/// Normalized coordinates. `break_before` leaves a visible gap in missing samples.
#[derive(Clone, Copy, Debug)]
pub struct ChartPoint {
    pub x: f32,
    pub y: f32,
    pub break_before: bool,
}

pub fn line_chart(samples: Vec<ChartPoint>, color: u32, height: f32) -> Div {
    div().h(px(height.max(0.))).w_full().child(
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let position = |x: f32, y: f32| {
                    bounds.origin
                        + point(
                            bounds.size.width * normalized(x),
                            bounds.size.height * (1. - normalized(y)),
                        )
                };
                let mut grid = PathBuilder::stroke(px(1.));
                for index in 0..=4 {
                    let y = index as f32 / 4.;
                    grid.move_to(position(0., y));
                    grid.line_to(position(1., y));
                }
                if let Ok(path) = grid.build() {
                    window.paint_path(path, rgb(theme::BORDER).alpha(0.65));
                }
                let mut line = PathBuilder::stroke(px(1.8));
                let mut previous_valid = false;
                for sample in &samples {
                    if !sample.x.is_finite() || !sample.y.is_finite() {
                        previous_valid = false;
                        continue;
                    }
                    let p = position(sample.x, sample.y);
                    if !previous_valid || sample.break_before {
                        line.move_to(p);
                    } else {
                        line.line_to(p);
                    }
                    previous_valid = true;
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, rgb(color));
                }
                // A lone observation is a point, not an invented full-width trend.
                for sample in samples
                    .iter()
                    .filter(|s| s.x.is_finite() && s.y.is_finite())
                    .rev()
                    .take(1)
                {
                    let p = position(sample.x, sample.y);
                    let mut marker = PathBuilder::stroke(px(3.));
                    marker.move_to(p - point(px(1.), px(0.)));
                    marker.line_to(p + point(px(1.), px(0.)));
                    if let Ok(path) = marker.build() {
                        window.paint_path(path, rgb(color));
                    }
                }
            },
        )
        .size_full(),
    )
}

/// Compact histogram retained for consumers that need peak-based downsampling.
pub fn bar_chart(samples: &[f32], color: u32, height: f32) -> Div {
    div()
        .flex()
        .items_end()
        .gap(px(3.))
        .h(px(height.max(0.)))
        .w_full()
        .children(chart_samples(samples).into_iter().map(|sample| {
            div()
                .flex_1()
                .min_w(px(1.))
                .h(px(height.max(0.) * sample))
                .bg(rgb(color))
        }))
}

fn chart_samples(samples: &[f32]) -> Vec<f32> {
    // Bound layout work and preserve peaks when a time window contains many samples.
    samples
        .chunks(samples.len().div_ceil(64).max(1))
        .map(|chunk| chunk.iter().copied().map(normalized).fold(0., f32::max))
        .collect()
}

fn normalized(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0., 1.)
    } else {
        0.
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn chart_downsampling_is_bounded_and_preserves_peaks() {
        let mut samples = vec![0.1; 300];
        samples[120] = 0.9;
        let values = super::chart_samples(&samples);
        assert!(values.len() <= 64);
        assert!(values.contains(&0.9));
        assert!(super::chart_samples(&[]).is_empty());
    }

    #[test]
    fn chart_values_are_bounded_and_finite() {
        for (input, expected) in [
            (-1., 0.),
            (0.5, 0.5),
            (10., 1.),
            (f32::NAN, 0.),
            (f32::INFINITY, 0.),
        ] {
            assert_eq!(super::normalized(input), expected);
        }
    }
}
