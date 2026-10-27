use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

use ruffle_render::backend::RenderBackend;
use ruffle_render_wgpu::{backend::WgpuRenderBackend, descriptors::Descriptors, wgpu};

use super::frames::{FrameLease, FrameTarget, Frames};
use super::{PlayerError, Viewport};

pub struct Surface(Target);

#[derive(Clone)]
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl PartialEq for Gpu {
    fn eq(&self, other: &Gpu) -> bool {
        // wgpu compares devices by their id within an instance, and a new instance reuses ids.
        self.instance == other.instance && self.device == other.device
    }
}

enum Target {
    #[cfg(target_os = "macos")]
    MetalLayer(objc2::rc::Retained<objc2::runtime::AnyObject>),
    #[cfg(not(target_os = "macos"))]
    Raw {
        window: raw_window_handle::RawWindowHandle,
        display: raw_window_handle::RawDisplayHandle,
    },
    Textures {
        gpu: Gpu,
        frames: Frames,
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

    pub fn textures(gpu: Gpu, wake: impl Fn() + Send + Sync + 'static) -> Surface {
        Surface(Target::Textures {
            gpu,
            frames: Frames::new(wake),
        })
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
            Target::Textures { gpu, frames } => Target::Textures {
                gpu: gpu.clone(),
                frames: frames.clone(),
            },
        })
    }

    pub(crate) fn lease(&self) -> Option<FrameLease> {
        match &self.0 {
            #[cfg(target_os = "macos")]
            Target::MetalLayer(_) => None,
            #[cfg(not(target_os = "macos"))]
            Target::Raw { .. } => None,
            Target::Textures { frames, .. } => frames.lease(),
        }
    }

    pub(crate) fn wants_frame(&self) -> bool {
        match &self.0 {
            #[cfg(target_os = "macos")]
            Target::MetalLayer(_) => true,
            #[cfg(not(target_os = "macos"))]
            Target::Raw { .. } => true,
            Target::Textures { frames, .. } => frames.wants_frame(),
        }
    }

    pub(crate) fn renderer(
        &self,
        viewport: Viewport,
    ) -> Result<Box<dyn RenderBackend>, PlayerError> {
        let window = match &self.0 {
            #[cfg(target_os = "macos")]
            Target::MetalLayer(layer) => wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                objc2::rc::Retained::as_ptr(layer).cast_mut().cast(),
            ),
            #[cfg(not(target_os = "macos"))]
            Target::Raw { window, display } => wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(*display),
                raw_window_handle: *window,
            },
            Target::Textures { gpu, frames } => {
                let descriptors = Descriptors::new(
                    gpu.instance.clone(),
                    gpu.adapter.clone(),
                    gpu.device.clone(),
                    gpu.queue.clone(),
                );
                let target = FrameTarget::new(
                    frames.clone(),
                    &gpu.device,
                    viewport.width(),
                    viewport.height(),
                );
                return boxed(WgpuRenderBackend::new(Arc::new(descriptors), target));
            }
        };
        boxed(unsafe {
            WgpuRenderBackend::for_window_unsafe(
                window,
                (viewport.width(), viewport.height()),
                wgpu::Backends::PRIMARY,
                wgpu::PowerPreference::HighPerformance,
                None,
            )
        })
    }
}

fn boxed(
    backend: Result<impl RenderBackend + 'static, Box<dyn std::error::Error>>,
) -> Result<Box<dyn RenderBackend>, PlayerError> {
    match backend {
        Ok(backend) => Ok(Box::new(backend)),
        Err(error) => Err(PlayerError::Renderer(error.to_string())),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use super::*;

    fn ready<T>(future: impl Future<Output = T>) -> T {
        match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("the noop backend answers immediately"),
        }
    }

    pub(crate) fn noop_gpu() -> Gpu {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::NOOP,
            backend_options: wgpu::BackendOptions {
                noop: wgpu::NoopBackendOptions::enabled(),
                ..Default::default()
            },
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = ready(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = ready(adapter.request_device(&Default::default())).unwrap();
        Gpu {
            instance,
            adapter,
            device,
            queue,
        }
    }

    #[test]
    fn a_device_from_a_recreated_instance_is_another_gpu() {
        let gpu = noop_gpu();
        assert!(gpu == gpu.clone());
        assert!(gpu != noop_gpu());
    }
}
