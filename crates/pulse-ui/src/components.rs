// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: components.rs
// @Software: resource-pulse

use crate::theme;
use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, SharedString, Stateful, Window, div, prelude::*,
    px, relative, rgb,
};

pub fn card() -> Div {
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(theme::BORDER))
        .bg(rgb(theme::SURFACE))
        .p(px(20.))
}

pub fn caption(text: impl Into<SharedString>) -> Div {
    div()
        .text_xs()
        .text_color(rgb(theme::MUTED))
        .child(text.into())
}

pub fn heading(text: impl Into<SharedString>) -> Div {
    div()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.into())
}

pub fn section_header(title: impl Into<SharedString>, detail: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .mb(px(14.))
        .child(heading(title))
        .child(caption(detail))
}

pub fn status(text: impl Into<SharedString>, color: u32) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(div().size(px(6.)).rounded_full().bg(rgb(color)))
        .child(caption(text))
}

pub fn progress(value: f32, color: u32) -> Div {
    let value = if value.is_finite() {
        value.clamp(0., 1.)
    } else {
        0.
    };
    div()
        .h(px(4.))
        .w_full()
        .rounded(px(2.))
        .bg(rgb(theme::HOVER))
        .overflow_hidden()
        .child(div().h_full().w(relative(value)).bg(rgb(color)))
}

/// The explicit id keeps hover, active and click state stable across renders.
pub fn button(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .px(px(12.))
        .py(px(7.))
        .rounded(px(4.))
        .border_1()
        .border_color(rgb(if selected { 0xb6c8e3 } else { theme::BORDER }))
        .bg(rgb(if selected {
            theme::TINT
        } else {
            theme::SURFACE
        }))
        .text_color(rgb(if selected { theme::ACCENT } else { theme::INK }))
        .text_xs()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(theme::HOVER)))
        .active(|style| style.bg(rgb(theme::TINT)))
        .on_click(on_click)
        .child(text.into())
}
