use std::io;
use std::time::Duration;

pub(crate) use platform::ThreadCpu;

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    pub(crate) struct ThreadCpu(libc::clockid_t);

    impl ThreadCpu {
        pub(crate) fn current() -> io::Result<Self> {
            let mut clock = 0;
            let result = unsafe { libc::pthread_getcpuclockid(libc::pthread_self(), &mut clock) };
            if result != 0 {
                return Err(io::Error::from_raw_os_error(result));
            }
            Ok(Self(clock))
        }

        pub(crate) fn elapsed(&self) -> io::Result<Duration> {
            let mut time = std::mem::MaybeUninit::uninit();
            if unsafe { libc::clock_gettime(self.0, time.as_mut_ptr()) } != 0 {
                return Err(io::Error::last_os_error());
            }
            let time = unsafe { time.assume_init() };
            Ok(Duration::new(time.tv_sec as u64, time.tv_nsec as u32))
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    pub(crate) struct ThreadCpu(libc::mach_port_t);

    unsafe extern "C" {
        fn mach_thread_self() -> libc::mach_port_t;
        static mach_task_self_: libc::mach_port_t;
        fn mach_port_deallocate(
            task: libc::mach_port_t,
            name: libc::mach_port_t,
        ) -> libc::kern_return_t;
    }

    impl ThreadCpu {
        pub(crate) fn current() -> io::Result<Self> {
            let port = unsafe { mach_thread_self() };
            if port == 0 {
                return Err(io::Error::other("mach_thread_self returned a null port"));
            }
            Ok(Self(port))
        }

        pub(crate) fn elapsed(&self) -> io::Result<Duration> {
            let mut info = std::mem::MaybeUninit::<libc::thread_basic_info>::uninit();
            let mut count = libc::THREAD_BASIC_INFO_COUNT;
            let result = unsafe {
                libc::thread_info(
                    self.0,
                    libc::THREAD_BASIC_INFO as libc::thread_flavor_t,
                    info.as_mut_ptr().cast(),
                    &mut count,
                )
            };
            if result != libc::KERN_SUCCESS {
                return Err(io::Error::other(format!("thread_info failed: {result}")));
            }
            let info = unsafe { info.assume_init() };
            Ok(
                Duration::from_secs(
                    info.user_time.seconds as u64 + info.system_time.seconds as u64,
                ) + Duration::from_micros(
                    info.user_time.microseconds as u64 + info.system_time.microseconds as u64,
                ),
            )
        }
    }

    impl Drop for ThreadCpu {
        fn drop(&mut self) {
            unsafe { mach_port_deallocate(mach_task_self_, self.0) };
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};

    pub(crate) struct ThreadCpu(OwnedHandle);

    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThreadId() -> u32;
        fn OpenThread(access: u32, inherit: i32, id: u32) -> RawHandle;
        fn GetThreadTimes(
            thread: RawHandle,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }

    impl ThreadCpu {
        pub(crate) fn current() -> io::Result<Self> {
            const THREAD_QUERY_LIMITED_INFORMATION: u32 = 0x0800;
            let handle =
                unsafe { OpenThread(THREAD_QUERY_LIMITED_INFORMATION, 0, GetCurrentThreadId()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            Ok(Self(unsafe { OwnedHandle::from_raw_handle(handle) }))
        }

        pub(crate) fn elapsed(&self) -> io::Result<Duration> {
            let mut creation = FileTime::default();
            let mut exit = FileTime::default();
            let mut kernel = FileTime::default();
            let mut user = FileTime::default();
            if unsafe {
                GetThreadTimes(
                    self.0.as_raw_handle(),
                    &mut creation,
                    &mut exit,
                    &mut kernel,
                    &mut user,
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            let ticks = |time: FileTime| (u64::from(time.high) << 32) | u64::from(time.low);
            let ticks = ticks(kernel) + ticks(user);
            Ok(Duration::new(
                ticks / 10_000_000,
                (ticks % 10_000_000) as u32 * 100,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn the_cpu_clock_reads_its_owner_from_another_thread() {
        let (ready, clock) = mpsc::channel();
        let (work, incoming) = mpsc::channel();
        let (done, completed) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let owner = std::thread::spawn(move || {
            let clock = ThreadCpu::current().unwrap();
            ready.send(clock).unwrap();
            incoming.recv().unwrap();
            let clock = ThreadCpu::current().unwrap();
            let began = clock.elapsed().unwrap();
            while clock.elapsed().unwrap() - began < Duration::from_millis(30) {
                std::hint::black_box((0..1000).sum::<u64>());
            }
            done.send(()).unwrap();
            held.recv().unwrap();
        });
        let clock = clock.recv().unwrap();
        let before = clock.elapsed().unwrap();
        work.send(()).unwrap();
        completed.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(clock.elapsed().unwrap() - before >= Duration::from_millis(30));
        release.send(()).unwrap();
        owner.join().unwrap();
    }
}
