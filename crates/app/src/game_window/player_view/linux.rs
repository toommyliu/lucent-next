use std::sync::Arc;

use gpui::{Bounds, Pixels, Window};
use lucent_host::{FrameLease, Gpu, Session, Surface, wgpu};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the window has no wgpu device to share")]
    NoDevice,
}

pub(super) struct NativeView {
    gpu: Gpu,
    wake: async_channel::Sender<()>,
}

impl NativeView {
    pub(super) fn attach(
        window: &Window,
        wake: async_channel::Sender<()>,
    ) -> Result<NativeView, Error> {
        let gpu = gpu(window).ok_or(Error::NoDevice)?;
        Ok(NativeView { gpu, wake })
    }

    pub(super) fn surface(&self) -> Surface {
        let wake = self.wake.clone();
        Surface::textures(self.gpu.clone(), move || {
            let _ = wake.try_send(());
        })
    }

    pub(super) fn replaced_device(&mut self, window: &Window) -> Option<Surface> {
        let gpu = gpu(window)?;
        if gpu == self.gpu {
            return None;
        }
        self.gpu = gpu;
        Some(self.surface())
    }

    pub(super) fn set_frame(&self, _bounds: Bounds<Pixels>, _scale: f32) {}

    pub(super) fn paint(&self, session: &Session, bounds: Bounds<Pixels>, window: &mut Window) {
        if let Some(frame) = session.frame() {
            window.paint_surface(bounds, Arc::new(Frame(frame)));
        }
    }
}

fn gpu(window: &Window) -> Option<Gpu> {
    let gpui::WgpuDevice {
        instance,
        adapter,
        device,
        queue,
    } = window.wgpu_device()?;
    Some(Gpu {
        instance,
        adapter,
        device,
        queue,
    })
}

#[derive(Debug)]
struct Frame(FrameLease);

impl gpui::SurfaceTexture for Frame {
    fn view(&self) -> &wgpu::TextureView {
        self.0.view()
    }
}
