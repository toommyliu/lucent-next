use gpui::{Div, ParentElement as _, Styled as _, div, px, rgb, rgba};

pub(super) fn overlay() -> [Div; 2] {
    [
        div()
            .absolute()
            .top(px(48.))
            .left(px(32.))
            .w(px(180.))
            .h(px(80.))
            .bg(rgb(0xff0000)),
        div()
            .absolute()
            .top(px(152.))
            .left(px(32.))
            .w(px(320.))
            .p(px(16.))
            .bg(rgba(0x18203080))
            .text_color(rgb(0xffffff))
            .child("A half-transparent card over the game"),
    ]
}
