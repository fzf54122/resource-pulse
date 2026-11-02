// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: icon.rs
// @Software: resource-pulse

use gpui::{Div, PathBuilder, canvas, div, point, prelude::*, px, rgb};

#[derive(Clone, Copy)]
pub enum Icon {
    Overview,
    Processes,
    History,
    Settings,
    Pulse,
}

pub fn icon(icon: Icon, color: u32) -> Div {
    let paths: &[&[(f32, f32)]] = match icon {
        Icon::Overview => &[
            &[(3., 3.), (9., 3.), (9., 9.), (3., 9.), (3., 3.)],
            &[(13., 3.), (19., 3.), (19., 9.), (13., 9.), (13., 3.)],
            &[(3., 13.), (9., 13.), (9., 19.), (3., 19.), (3., 13.)],
            &[(13., 13.), (19., 13.), (19., 19.), (13., 19.), (13., 13.)],
        ],
        Icon::Processes => &[
            &[(3., 5.), (19., 5.)],
            &[(3., 11.), (19., 11.)],
            &[(3., 17.), (19., 17.)],
            &[(7., 3.), (7., 19.)],
        ],
        Icon::History => &[
            &[(3., 3.), (3., 19.), (20., 19.)],
            &[(5., 14.), (9., 9.), (13., 12.), (19., 5.)],
        ],
        Icon::Settings => &[
            &[(3., 5.), (19., 5.)],
            &[(3., 11.), (19., 11.)],
            &[(3., 17.), (19., 17.)],
            &[(8., 2.), (8., 8.)],
            &[(15., 8.), (15., 14.)],
            &[(7., 14.), (7., 20.)],
        ],
        Icon::Pulse => &[&[
            (1., 11.),
            (6., 11.),
            (9., 4.),
            (13., 18.),
            (16., 11.),
            (21., 11.),
        ]],
    };
    div().size(px(18.)).flex_shrink_0().child(
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let mut path = PathBuilder::stroke(px(1.4));
                for vertices in paths {
                    for (index, &(x, y)) in vertices.iter().enumerate() {
                        let p = bounds.origin
                            + point(
                                bounds.size.width * (x / 22.),
                                bounds.size.height * (y / 22.),
                            );
                        if index == 0 {
                            path.move_to(p);
                        } else {
                            path.line_to(p);
                        }
                    }
                }
                if let Ok(path) = path.build() {
                    window.paint_path(path, rgb(color));
                }
            },
        )
        .size_full(),
    )
}
