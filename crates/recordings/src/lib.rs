//! Internal recordings on DJI Mic transmitters.
//!
//! Transmitters don't send their recordings over the receiver link. With a
//! transmitter in the charging case (or on its magnetic cable) and the case
//! plugged in over USB-C, each transmitter mounts as its own USB drive. This
//! crate finds those drives ([`Scanner`]), describes the takes on them, and
//! copies them off with verification ([`import`]).

pub mod import;
mod scan;
pub mod wav;
#[cfg(feature = "wireless")]
pub mod wireless;

pub use import::{delete_imported, dest_path, mark_imported, plan, run, Deleted, Failure, Job, Outcome, Progress};
pub use scan::{build_transmitter, mount_points, Entry, FileKind, RecordingFile, Scanner, Take, Transmitter};
pub use wav::WavInfo;

#[cfg(test)]
mod testdir {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A temporary directory removed on drop.
    pub struct TestDir(PathBuf);

    impl TestDir {
        pub fn new(name: &str) -> Self {
            static N: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!(
                "djimic-recordings-{name}-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
