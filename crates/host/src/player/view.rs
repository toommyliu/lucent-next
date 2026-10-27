
use std::num::NonZeroU32;
use std::ops::Range;

use ruffle_core::events::{ImeEvent, MouseWheelDelta, TextControlCode};
use ruffle_core::{PlayerEvent, ViewportDimensions};

#[derive(Clone, Debug, PartialEq)]
pub enum ViewEvent {
    Resized(Viewport),
    MouseMove(Point),
    MouseDown { at: Point, button: MouseButton, clicks: usize },
    MouseUp { at: Point, button: MouseButton },
    MouseLeave,
    Wheel(WheelDelta),
    KeyDown(Key),
    KeyUp(Key),
    Text(String),
    Ime(Ime),
    Focus(bool),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    width: NonZeroU32,
    height: NonZeroU32,
    scale: f64,
}

impl Viewport {
    pub fn new(logical_width: f32, logical_height: f32, scale: f32) -> Option<Viewport> {
        todo!()
    }

    pub(crate) fn width(self) -> u32 {
        self.width.get()
    }

    pub(crate) fn height(self) -> u32 {
        self.height.get()
    }

    pub(crate) fn scale(self) -> f64 {
        self.scale
    }

    fn dimensions(self) -> ViewportDimensions {
        ViewportDimensions { width: self.width(), height: self.height(), scale_factor: self.scale }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelDelta {
    Lines(f32),
    Pixels(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub code: KeyCode,
    pub modifiers: Modifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Named(NamedKey),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamedKey {
    Enter, Tab, Backspace, Delete, Escape, Insert,
    Home, End, PageUp, PageDown,
    ArrowLeft, ArrowRight, ArrowUp, ArrowDown,
    Shift, Control, Alt, Command, CapsLock,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub command: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ime {
    Preedit { text: String, cursor: Option<Range<usize>> },
    Commit(String),
}

#[derive(Debug)]
pub(crate) enum Effect {
    Event(PlayerEvent),
    Resize(Viewport),
    MouseInStage(bool),
}

pub(crate) fn translate(event: ViewEvent, viewport: Viewport, out: &mut Vec<Effect>) {
    todo!()
}

fn descriptor(key: Key) -> ruffle_core::events::KeyDescriptor {
    todo!()
}

fn text_control(key: Key) -> Option<TextControlCode> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_events_translate_to_literal_player_effects() {
        let cases = [
            (ViewEvent::MouseDown { at: Point { x: 10.0, y: 20.5 }, button: MouseButton::Left, clicks: 2 }, "[Event(MouseDown { x: 20.0, y: 41.0, button: Left, index: Some(1) })]"),
            (ViewEvent::KeyDown(Key { code: KeyCode::Named(NamedKey::Backspace), modifiers: Modifiers::default() }), "[Event(KeyDown { key: KeyDescriptor { physical_key: Unknown, logical_key: Named(Backspace), key_location: Standard } }), Event(TextControl { code: Backspace })]"),
            (ViewEvent::Text("é\n".into()), "[Event(TextInput { codepoint: 'é' })]"),
            (ViewEvent::Focus(false), "[Event(FocusLost)]"),
        ];
        for (event, expected) in cases {
            let mut effects = Vec::new();
            translate(event, Viewport::new(1024.0, 740.0, 2.0).unwrap(), &mut effects);
            assert_eq!(format!("{effects:?}"), expected);
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn command_c_is_a_key_and_copy() {
        let mut effects = Vec::new();
        translate(ViewEvent::KeyDown(Key { code: KeyCode::Char('c'), modifiers: Modifiers { command: true, ..Default::default() } }), Viewport::new(1024.0, 740.0, 2.0).unwrap(), &mut effects);
        assert_eq!(format!("{effects:?}"), "[Event(KeyDown { key: KeyDescriptor { physical_key: Unknown, logical_key: Character('c'), key_location: Standard } }), Event(TextControl { code: Copy })]");
    }

    #[test]
    fn viewport_validates_and_scales_logical_dimensions() {
        assert_eq!(Viewport::new(1024.0, 740.0, 2.0).map(|v| (v.width(), v.height(), v.scale())), Some((2048, 1480, 2.0)));
        for (width, height, scale) in [(0.0, 740.0, 2.0), (-1.0, 10.0, 1.0), (10.0, 0.0, 1.0), (10.0, 10.0, f32::NAN), (10.0, 10.0, 0.0)] {
            assert_eq!(Viewport::new(width, height, scale), None);
        }
    }
}
