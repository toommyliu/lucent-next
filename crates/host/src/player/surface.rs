#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

use ruffle_render_wgpu::{backend::WgpuRenderBackend, target::SwapChainTarget, wgpu};

use super::{PlayerError, Viewport};

pub struct Surface(Target);

enum Target {
    #[cfg(target_os = "macos")]
    MetalLayer(objc2::rc::Retained<objc2::runtime::AnyObject>),
    #[cfg(not(target_os = "macos"))]
    Raw {
        window: raw_window_handle::RawWindowHandle,
        display: raw_window_handle::RawDisplayHandle,
    },
}

// wgpu accepts a retained CAMetalLayer off the main thread. Raw targets must outlive their session.
unsafe impl Send for Surface {}

impl Surface {
    /// # Safety
    /// `layer` must point to a live CAMetalLayer. This surface takes its own retain.
    #[cfg(target_os = "macos")]
    pub unsafe fn from_metal_layer(layer: NonNull<c_void>) -> Surface {
        let layer = unsafe { objc2::rc::Retained::retain(layer.cast().as_ptr()) }.unwrap();
        Surface(Target::MetalLayer(layer))
    }

    /// # Safety
    /// The window and display must remain valid until the session has stopped drawing.
    #[cfg(not(target_os = "macos"))]
    pub unsafe fn from_raw(
        window: raw_window_handle::RawWindowHandle,
        display: raw_window_handle::RawDisplayHandle,
    ) -> Surface {
        Surface(Target::Raw { window, display })
    }

    pub(crate) fn share(&self) -> Surface {
        Surface(match &self.0 {
            #[cfg(target_os = "macos")]
            Target::MetalLayer(layer) => Target::MetalLayer(layer.clone()),
            #[cfg(not(target_os = "macos"))]
            Target::Raw { window, display } => Target::Raw {
                window: *window,
                display: *display,
            },
        })
    }

    pub(crate) fn renderer(
        &self,
        viewport: Viewport,
    ) -> Result<WgpuRenderBackend<SwapChainTarget>, PlayerError> {
        let target = match &self.0 {
            #[cfg(target_os = "macos")]
            Target::MetalLayer(layer) => wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                objc2::rc::Retained::as_ptr(layer).cast_mut().cast(),
            ),
            #[cfg(not(target_os = "macos"))]
            Target::Raw { window, display } => wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(*display),
                raw_window_handle: *window,
            },
        };
        unsafe {
            WgpuRenderBackend::for_window_unsafe(
                target,
                (viewport.width(), viewport.height()),
                wgpu::Backends::PRIMARY,
                wgpu::PowerPreference::HighPerformance,
                None,
            )
        }
        .map_err(|error| PlayerError::Renderer(error.to_string()))
    }
}
