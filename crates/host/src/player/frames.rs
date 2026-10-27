use std::fmt;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use ruffle_render_wgpu::target::{RenderTarget, RenderTargetFrame};
use ruffle_render_wgpu::wgpu;

const SLOTS: usize = 2;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[derive(Clone)]
pub(crate) struct Frames(Arc<Mailbox>);

struct Mailbox {
    latest: Mutex<Option<Published>>,
    wake: Box<dyn Fn() + Send + Sync>,
}

struct Published {
    frame: Arc<Frame>,
    shown: bool,
}

#[derive(Debug)]
pub(crate) struct Frame {
    view: wgpu::TextureView,
    free: mpsc::Sender<wgpu::TextureView>,
}

/// Drop it only after every read of its view has been submitted to the queue the player
/// renders on. Queue order then keeps the player's next draw into this texture behind them.
#[derive(Debug)]
pub struct FrameLease(Arc<Frame>);

#[derive(Debug)]
pub(crate) struct FrameTarget {
    frames: Frames,
    free: mpsc::Receiver<wgpu::TextureView>,
    returns: mpsc::Sender<wgpu::TextureView>,
    width: u32,
    height: u32,
}

impl Frames {
    pub(crate) fn new(wake: impl Fn() + Send + Sync + 'static) -> Frames {
        Frames(Arc::new(Mailbox {
            latest: Mutex::new(None),
            wake: Box::new(wake),
        }))
    }

    pub(crate) fn lease(&self) -> Option<FrameLease> {
        let mut latest = self.0.latest.lock().unwrap();
        let published = latest.as_mut()?;
        published.shown = true;
        Some(FrameLease(published.frame.clone()))
    }

    pub(crate) fn wants_frame(&self) -> bool {
        self.0
            .latest
            .lock()
            .unwrap()
            .as_ref()
            .is_none_or(|published| published.shown)
    }

    fn publish(&self, frame: Frame) {
        *self.0.latest.lock().unwrap() = Some(Published {
            frame: Arc::new(frame),
            shown: false,
        });
        (self.0.wake)();
    }
}

impl fmt::Debug for Frames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frames").finish_non_exhaustive()
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        let _ = self.free.send(self.view.clone());
    }
}

impl RenderTargetFrame for Frame {
    fn into_view(self) -> wgpu::TextureView {
        self.view.clone()
    }

    fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

impl FrameLease {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.0.view
    }
}

impl FrameTarget {
    pub(crate) fn new(
        frames: Frames,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> FrameTarget {
        let (returns, free) = textures(device, width, height);
        FrameTarget {
            frames,
            free,
            returns,
            width,
            height,
        }
    }
}

impl RenderTarget for FrameTarget {
    type Frame = Frame;

    fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if (width, height) == (self.width, self.height) {
            return;
        }
        (self.returns, self.free) = textures(device, width, height);
        self.width = width;
        self.height = height;
    }

    fn format(&self) -> wgpu::TextureFormat {
        FORMAT
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn get_next_texture(&mut self) -> Option<Frame> {
        let view = self.free.try_recv().ok()?;
        Some(Frame {
            view,
            free: self.returns.clone(),
        })
    }

    fn submit<I: IntoIterator<Item = wgpu::CommandBuffer>>(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        command_buffers: I,
        frame: Frame,
    ) -> wgpu::SubmissionIndex {
        let index = queue.submit(command_buffers);
        self.frames.publish(frame);
        index
    }
}

fn textures(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (
    mpsc::Sender<wgpu::TextureView>,
    mpsc::Receiver<wgpu::TextureView>,
) {
    let (returns, free) = mpsc::channel();
    for _ in 0..SLOTS {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("player frame"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let _ = returns.send(texture.create_view(&Default::default()));
    }
    (returns, free)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    struct Noop {
        device: wgpu::Device,
        queue: wgpu::Queue,
    }

    impl Noop {
        fn new() -> Noop {
            let (device, queue) = wgpu::Device::noop(&Default::default());
            Noop { device, queue }
        }

        fn target(&self, frames: &Frames, width: u32, height: u32) -> FrameTarget {
            FrameTarget::new(frames.clone(), &self.device, width, height)
        }

        fn draw(&self, target: &mut FrameTarget) -> Option<wgpu::TextureView> {
            let frame = target.get_next_texture()?;
            let view = frame.view.clone();
            target.submit(&self.device, &self.queue, [], frame);
            Some(view)
        }
    }

    fn size(view: &wgpu::TextureView) -> (u32, u32) {
        (view.texture().width(), view.texture().height())
    }

    #[test]
    fn a_leased_texture_is_not_drawn_into_until_its_lease_drops() {
        let gpu = Noop::new();
        let frames = Frames::new(|| {});
        let mut target = gpu.target(&frames, 4, 4);
        let first = gpu.draw(&mut target).unwrap();
        let lease = frames.lease().unwrap();
        let second = gpu.draw(&mut target).unwrap();
        assert_ne!(second, first);
        assert!(target.get_next_texture().is_none());
        drop(lease);
        assert_eq!(gpu.draw(&mut target), Some(first));
    }

    #[test]
    fn the_published_frame_is_not_drawn_into() {
        let gpu = Noop::new();
        let frames = Frames::new(|| {});
        let mut target = gpu.target(&frames, 4, 4);
        let published = gpu.draw(&mut target).unwrap();
        let drawing = target.get_next_texture().unwrap();
        assert_ne!(drawing.view, published);
        assert!(target.get_next_texture().is_none());
    }

    #[test]
    fn the_window_leases_the_latest_frame_until_another_is_published() {
        let gpu = Noop::new();
        let wakes = Arc::new(AtomicUsize::new(0));
        let frames = Frames::new({
            let wakes = wakes.clone();
            move || {
                wakes.fetch_add(1, Ordering::Relaxed);
            }
        });
        let mut target = gpu.target(&frames, 4, 4);
        assert!(frames.lease().is_none());
        gpu.draw(&mut target).unwrap();
        let latest = gpu.draw(&mut target).unwrap();
        assert_eq!(wakes.load(Ordering::Relaxed), 2);
        assert_eq!(frames.lease().unwrap().view(), &latest);
        assert_eq!(frames.lease().unwrap().view(), &latest);
    }

    #[test]
    fn the_player_waits_until_the_window_has_shown_its_last_frame() {
        let gpu = Noop::new();
        let frames = Frames::new(|| {});
        let mut target = gpu.target(&frames, 4, 4);
        assert!(frames.wants_frame());
        gpu.draw(&mut target).unwrap();
        assert!(!frames.wants_frame());
        let _lease = frames.lease();
        assert!(frames.wants_frame());
    }

    #[test]
    fn resizing_replaces_the_pair_and_keeps_old_leases_valid() {
        let gpu = Noop::new();
        let frames = Frames::new(|| {});
        let mut target = gpu.target(&frames, 4, 4);
        gpu.draw(&mut target).unwrap();
        let old = frames.lease().unwrap();
        target.resize(&gpu.device, 8, 6);
        let first = gpu.draw(&mut target).unwrap();
        let second = gpu.draw(&mut target).unwrap();
        assert_eq!(size(old.view()), (4, 4));
        assert_eq!([size(&first), size(&second)], [(8, 6), (8, 6)]);
        drop(old);
        let drawing = target.get_next_texture().unwrap();
        assert_eq!(drawing.view, first);
        assert!(target.get_next_texture().is_none());
    }

    #[test]
    fn resizing_to_the_same_size_keeps_the_pair() {
        let gpu = Noop::new();
        let frames = Frames::new(|| {});
        let mut target = gpu.target(&frames, 4, 4);
        gpu.draw(&mut target).unwrap();
        let _lease = frames.lease().unwrap();
        gpu.draw(&mut target).unwrap();
        target.resize(&gpu.device, 4, 4);
        assert!(target.get_next_texture().is_none());
    }
}
