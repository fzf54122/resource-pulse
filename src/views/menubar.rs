// @Time    : 2026/11/2 00:00
// @Author  : fzf
// @FileName: menubar.rs
// @Software: resource-pulse

use super::*;
use crate::{brand, commands::MenuGroup};
use pulse_ui::gpui::{Stateful, img};

impl ResourcePulse {
    pub(super) fn menubar(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .items_center()
            .h(px(36.))
            .px(px(12.))
            .gap_1()
            .flex_shrink_0()
            .bg(rgb(theme::SURFACE))
            .border_b_1()
            .border_color(rgb(BORDER))
            .children(MenuGroup::ALL.into_iter().map(|group| {
                let id = group.id();
                div()
                    .id(id)
                    .debug_selector(move || id.into())
                    .px(px(12.))
                    .py(px(5.))
                    .rounded(px(3.))
                    .text_xs()
                    .when(self.menu == Some(group), |view| view.bg(rgb(TINT)))
                    .cursor_pointer()
                    .hover(|view| view.bg(rgb(theme::HOVER)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.menu = if this.menu == Some(group) {
                            None
                        } else {
                            Some(group)
                        };
                        cx.notify();
                    }))
                    .child(group.title())
            }))
            .child(div().flex_1())
            .child(caption("此电脑"))
    }

    pub(super) fn menu_popup(&self, group: MenuGroup, cx: &mut Context<Self>) -> Stateful<Div> {
        card()
            .id("menu-popup")
            .absolute()
            .top(px(35.))
            .left(px(group.offset()))
            .w(px(252.))
            .p(px(4.))
            .shadow_md()
            .occlude()
            .on_click(|_, _, cx| cx.stop_propagation())
            .children(
                crate::menu::MenuState::new(&self.state, self.error.is_some())
                    .entries(group)
                    .into_iter()
                    .map(|entry| {
                        let id = entry.id;
                        div()
                            .id(id)
                            .debug_selector(move || id.into())
                            .flex()
                            .items_center()
                            .gap_2()
                            .px(px(10.))
                            .py(px(9.))
                            .rounded(px(3.))
                            .text_xs()
                            .when(!entry.enabled, |view| view.opacity(0.4))
                            .when(entry.enabled, |view| {
                                view.cursor_pointer().hover(|view| view.bg(rgb(TINT)))
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                if entry.enabled {
                                    this.execute_command(entry.command, window, cx);
                                }
                            }))
                            .child(div().w(px(14.)).child(if entry.checked { "✓" } else { "" }))
                            .child(div().flex_1().child(entry.label))
                            .child(caption(entry.shortcut))
                    }),
            )
    }

    pub(super) fn about_dialog(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("about-dialog")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0x172b3a).alpha(0.25))
            .occlude()
            .on_click(cx.listener(|this, _, _, cx| {
                this.about = false;
                cx.notify();
            }))
            .child(
                card()
                    .id("about-card")
                    .w(px(360.))
                    .p(px(28.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_3()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(img(brand::mascot()).size(px(96.)))
                    .child(heading(format!("Pulse {}", env!("CARGO_PKG_VERSION"))))
                    .child(caption("Pip · 小雨燕"))
                    .child(caption("系统资源监控"))
                    .child(self.control(
                        "close-about",
                        "关闭",
                        false,
                        |this, _| this.about = false,
                        cx,
                    )),
            )
    }
}
