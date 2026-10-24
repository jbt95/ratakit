//! Coalesced frame pacing for async render loops.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ratakit_core::MotionMode;
use tokio::sync::Notify;
use tokio::time::{self, Instant as TokioInstant};

const FPS_FULL: u32 = 60;
const FPS_REDUCED: u32 = 10;

fn frame_interval(motion: MotionMode) -> Duration {
    let fps = match motion {
        MotionMode::Full => FPS_FULL,
        MotionMode::Reduced => FPS_REDUCED,
    };
    Duration::from_nanos(1_000_000_000 / u64::from(fps))
}

struct Shared {
    pending: AtomicBool,
    notify: Notify,
}

/// Coalesces [`request`](Self::request) calls and rate-limits [`next_frame`](Self::next_frame).
///
/// [`request`](Self::request) is `Send + Sync` and may be called from any task. Use
/// [`next_frame`](Self::next_frame) in the render task. Default pacing is 60 fps; with
/// [`MotionMode::Reduced`] it is 10 fps.
#[derive(Clone)]
pub struct FrameScheduler {
    shared: Arc<Shared>,
    motion: MotionMode,
    last_frame: Option<TokioInstant>,
}

impl Default for FrameScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameScheduler {
    /// Scheduler with [`MotionMode::Full`] (60 fps cap).
    pub fn new() -> Self {
        Self::with_motion(MotionMode::Full)
    }

    /// Scheduler with an explicit motion mode.
    pub fn with_motion(motion: MotionMode) -> Self {
        Self {
            shared: Arc::new(Shared {
                pending: AtomicBool::new(false),
                notify: Notify::new(),
            }),
            motion,
            last_frame: None,
        }
    }

    /// Updates the frame rate cap (60 vs 10 fps).
    pub fn set_motion_mode(&mut self, motion: MotionMode) {
        self.motion = motion;
    }

    pub fn motion_mode(&self) -> MotionMode {
        self.motion
    }

    /// Request a redraw. Multiple calls before the next frame are coalesced into one.
    pub fn request(&self) {
        self.shared.pending.store(true, Ordering::Release);
        self.shared.notify.notify_waiters();
    }

    /// Wait until a frame should be drawn, then return the scheduled frame timestamp.
    pub async fn next_frame(&mut self) -> Instant {
        loop {
            while !self.shared.pending.load(Ordering::Acquire) {
                self.shared.notify.notified().await;
            }

            let interval = frame_interval(self.motion);
            let at = match self.last_frame {
                None => TokioInstant::now(),
                Some(last) => last + interval,
            };
            if TokioInstant::now() < at {
                time::sleep_until(at).await;
            }

            if self.shared.pending.swap(false, Ordering::AcqRel) {
                self.last_frame = Some(at);
                return at.into_std();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Instant as TokioInstant, advance, timeout};

    #[tokio::test(start_paused = true)]
    async fn first_frame_is_immediate() {
        let mut scheduler = FrameScheduler::new();
        scheduler.request();
        let start = TokioInstant::now();
        let at = scheduler.next_frame().await;
        assert_eq!(at, start.into_std());
    }

    #[tokio::test(start_paused = true)]
    async fn coalesces_multiple_requests() {
        let mut scheduler = FrameScheduler::new();
        scheduler.request();
        scheduler.request();
        scheduler.request();
        let _ = scheduler.next_frame().await;
        assert!(
            timeout(Duration::from_millis(1), scheduler.next_frame())
                .await
                .is_err()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn rate_limits_full_motion_to_60fps() {
        let mut scheduler = FrameScheduler::new();
        scheduler.request();
        let first = scheduler.next_frame().await;
        scheduler.request();
        let mut second = std::pin::pin!(scheduler.next_frame());
        advance(Duration::from_millis(10)).await;
        assert!(timeout(Duration::ZERO, second.as_mut()).await.is_err());
        advance(Duration::from_millis(10)).await;
        let second = second.await;
        assert_eq!(second - first, frame_interval(MotionMode::Full));
    }

    #[tokio::test(start_paused = true)]
    async fn reduced_motion_slower_pacing() {
        let mut scheduler = FrameScheduler::new();
        scheduler.set_motion_mode(MotionMode::Reduced);
        scheduler.request();
        let first = scheduler.next_frame().await;
        scheduler.request();
        advance(frame_interval(MotionMode::Reduced)).await;
        let second = scheduler.next_frame().await;
        assert_eq!(second - first, frame_interval(MotionMode::Reduced));
    }
}
