use std::cell::Cell;
use std::num::NonZeroIsize;
use std::sync::OnceLock;

use gpui::{Bounds, Pixels, Window};
use lucent_host::{Session, Surface};
use raw_window_handle::{
    HasWindowHandle, RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle,
};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, HTTRANSPARENT, RegisterClassW, SWP_NOACTIVATE,
    SWP_NOZORDER, SetWindowPos, WINDOW_EX_STYLE, WM_NCHITTEST, WNDCLASSW, WS_CHILD, WS_VISIBLE,
};
use windows::core::{PCWSTR, w};

const CLASS: PCWSTR = w!("LucentPlayerView");

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the window has no Win32 handle")]
    NoWin32Window,
    #[error("couldn't create the player's window: {0}")]
    CreateWindow(windows::core::Error),
}

pub(super) struct NativeView {
    handle: Win32WindowHandle,
    frame: Cell<Option<Frame>>,
}

impl NativeView {
    pub(super) fn attach(
        window: &Window,
        _wake: async_channel::Sender<()>,
    ) -> Result<NativeView, Error> {
        let handle = HasWindowHandle::window_handle(window).map_err(|_| Error::NoWin32Window)?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return Err(Error::NoWin32Window);
        };
        let instance =
            HINSTANCE::from(unsafe { GetModuleHandleW(None) }.map_err(Error::CreateWindow)?);
        register_class(instance).map_err(Error::CreateWindow)?;
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                CLASS,
                None,
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                0,
                0,
                Some(HWND(handle.hwnd.get() as _)),
                None,
                Some(instance),
                None,
            )
        }
        .map_err(Error::CreateWindow)?;
        let mut handle = Win32WindowHandle::new(NonZeroIsize::new(hwnd.0 as isize).unwrap());
        handle.hinstance = NonZeroIsize::new(instance.0 as isize);
        Ok(NativeView {
            handle,
            frame: Cell::new(None),
        })
    }

    fn hwnd(&self) -> HWND {
        HWND(self.handle.hwnd.get() as _)
    }

    pub(super) fn surface(&self) -> Surface {
        unsafe {
            Surface::from_raw(
                RawWindowHandle::Win32(self.handle),
                RawDisplayHandle::Windows(WindowsDisplayHandle::new()),
            )
        }
    }

    pub(super) fn replaced_device(&mut self, _window: &Window) -> Option<Surface> {
        None
    }

    pub(super) fn set_frame(&self, bounds: Bounds<Pixels>, scale: f32) {
        let Some(frame) = Frame::new(bounds, scale) else {
            return;
        };
        if self.frame.get() == Some(frame) {
            return;
        }
        let moved = unsafe {
            SetWindowPos(
                self.hwnd(),
                None,
                frame.x,
                frame.y,
                frame.width,
                frame.height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
        match moved {
            Ok(()) => self.frame.set(Some(frame)),
            Err(error) => tracing::warn!(%error, "couldn't move the player's window"),
        }
    }

    pub(super) fn paint(&self, _session: &Session, _bounds: Bounds<Pixels>, _window: &mut Window) {}
}

impl Drop for NativeView {
    fn drop(&mut self) {
        if let Err(error) = unsafe { DestroyWindow(self.hwnd()) } {
            tracing::warn!(%error, "couldn't destroy the player's window");
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Frame {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Frame {
    fn new(bounds: Bounds<Pixels>, scale: f32) -> Option<Frame> {
        let device = |logical: Pixels| f32::from(logical) * scale;
        let width = device(bounds.size.width).round() as i32;
        let height = device(bounds.size.height).round() as i32;
        (width > 0 && height > 0).then(|| Frame {
            x: device(bounds.origin.x).floor() as i32,
            y: device(bounds.origin.y).floor() as i32,
            width,
            height,
        })
    }
}

fn register_class(instance: HINSTANCE) -> windows::core::Result<()> {
    static REGISTERED: OnceLock<windows::core::Result<()>> = OnceLock::new();
    REGISTERED
        .get_or_init(|| {
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: CLASS,
                ..Default::default()
            };
            match unsafe { RegisterClassW(&class) } {
                0 => Err(windows::core::Error::from_win32()),
                _ => Ok(()),
            }
        })
        .clone()
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCHITTEST {
        return LRESULT(HTTRANSPARENT as isize);
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use gpui::{point, px, size};

    use super::*;

    fn game_area(width: f32, height: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(0.), px(30.)), size(px(width), px(height)))
    }

    #[test]
    fn the_player_window_covers_the_game_area_in_device_pixels() {
        let cases = [
            (game_area(1024., 768.), 1.0, Some((0, 30, 1024, 768))),
            (game_area(1024., 768.), 1.5, Some((0, 45, 1536, 1152))),
            (game_area(1024., 767.75), 1.25, Some((0, 37, 1280, 960))),
            (game_area(1000.3, 767.4), 1.75, Some((0, 52, 1751, 1343))),
            (game_area(0., 768.), 1.0, None),
            (game_area(1024., 0.2), 2.0, None),
        ];
        for (bounds, scale, expected) in cases {
            let frame = Frame::new(bounds, scale);
            assert_eq!(
                frame.map(|f| (f.x, f.y, f.width, f.height)),
                expected,
                "{bounds:?} at {scale}"
            );
        }
    }
}
