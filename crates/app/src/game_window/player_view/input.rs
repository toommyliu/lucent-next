
use gpui::{Bounds, Keystroke, Pixels, ScrollDelta};
use lucent_host::{Key, KeyCode, Modifiers, MouseButton, NamedKey, Point, ViewEvent, Viewport, WheelDelta};

pub(super) fn key(keystroke: &Keystroke) -> Option<Key> {
    todo!()
}

fn named(name: &str) -> Option<NamedKey> {
    todo!()
}

fn modifiers(modifiers: gpui::Modifiers) -> Modifiers {
    Modifiers {
        shift: modifiers.shift,
        control: modifiers.control,
        alt: modifiers.alt,
        command: modifiers.platform,
    }
}

pub(super) fn modifier_keys(before: gpui::Modifiers, after: gpui::Modifiers) -> Vec<ViewEvent> {
    todo!()
}

pub(super) fn button(button: gpui::MouseButton) -> MouseButton {
    match button {
        gpui::MouseButton::Left => MouseButton::Left,
        gpui::MouseButton::Right => MouseButton::Right,
        gpui::MouseButton::Middle => MouseButton::Middle,
        _ => MouseButton::Other,
    }
}

pub(super) fn point(position: gpui::Point<Pixels>, origin: gpui::Point<Pixels>) -> Point {
    let relative = position - origin;
    Point { x: relative.x.into(), y: relative.y.into() }
}

pub(super) fn wheel(delta: ScrollDelta) -> WheelDelta {
    match delta {
        ScrollDelta::Lines(lines) => WheelDelta::Lines(lines.y),
        ScrollDelta::Pixels(pixels) => WheelDelta::Pixels(pixels.y.into()),
    }
}

pub(super) fn viewport(bounds: Bounds<Pixels>, scale: f32) -> Option<Viewport> {
    Viewport::new(bounds.size.width.into(), bounds.size.height.into(), scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keystroke(key: &str, key_char: Option<&str>, modifiers: gpui::Modifiers) -> Keystroke {
        Keystroke { modifiers, key: key.into(), key_char: key_char.map(Into::into) }
    }

    #[test]
    fn shift_a_types_capital_a() {
        let shift = gpui::Modifiers { shift: true, ..Default::default() };
        assert_eq!(
            key(&keystroke("a", Some("A"), shift)),
            Some(Key { code: KeyCode::Char('A'), modifiers: Modifiers { shift: true, ..Default::default() } }),
        );
    }

    #[test]
    fn enter_is_a_named_key() {
        assert_eq!(
            key(&keystroke("enter", Some("\n"), gpui::Modifiers::default())).map(|k| k.code),
            Some(KeyCode::Named(NamedKey::Enter)),
        );
    }

    #[test]
    fn pressing_shift_is_a_shift_key_down() {
        let after = gpui::Modifiers { shift: true, ..Default::default() };
        assert_eq!(
            modifier_keys(gpui::Modifiers::default(), after),
            vec![ViewEvent::KeyDown(Key {
                code: KeyCode::Named(NamedKey::Shift),
                modifiers: Modifiers { shift: true, ..Default::default() },
            })],
        );
    }
}
