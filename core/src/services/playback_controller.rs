use crate::domain::Stream;
use crate::ports::{PlaybackListener, PlayerController};
use crate::services::StreamResolver;
use anyhow::Result as Res;
use parking_lot::RwLock;
use std::sync::Arc;

/**
* This trait provides the basic functionality that is available
* to control the playing of a provided stream.
*/
pub trait PlaybackController {
    fn play(&self, channel_id: &str);

    fn pause(&self);

    fn stop(&self);

    fn seek(&self, seconds: u32);
}

/**
* This represents the current state of the video stream.
*/
#[derive(Debug, Default, Clone, PartialEq)]
pub enum PlaybackState {
    #[default]
    Idle,
    Loading {
        channel_id: String,
        attempt: usize,
    },
    Playing {
        channel_id: String,
        stream_id: String,
    },
    Retrying {
        channel_id: String,
        attempt: usize,
        last_error: String,
    },
    Failed {
        channel_id: String,
        error: String,
    },
    Stopped,
}

/**
* This is a concreate implementation for the PlaybackController.
* It implements the common logic that all players will implement
* for managing and streaming a channel. It also contains the fallback
* resolution logic when
*/
#[derive(Debug)]
pub struct IpTvPlaybackController {
    pub stream_resolver: Arc<dyn StreamResolver>,
    pub player: Arc<dyn PlayerController>,
    candidates: RwLock<Vec<String>>,
    attempted_streams: RwLock<Vec<String>>,
    current_state: RwLock<PlaybackState>,
    last_error: RwLock<Option<String>>,
}

impl IpTvPlaybackController {
    pub fn new(
        stream_resolver: Arc<dyn StreamResolver>,
        player: Arc<dyn PlayerController>,
    ) -> Self {
        Self {
            stream_resolver,
            player,
            candidates: RwLock::new(vec![]),
            attempted_streams: Default::default(),
            current_state: Default::default(),
            last_error: RwLock::new(None),
        }
    }

    fn try_next_candidate(&self) -> Res<()> {
        // Pop the first candidate from the queue.
        let url = {
            let mut guard = self.candidates.write();
            if guard.is_empty() {
                return Err(anyhow::anyhow!("no more candidates"));
            }
            guard.remove(0)
        };

        // Record that we're attempting this URL.
        {
            let mut guard = self.attempted_streams.write();
            guard.push(url.clone());
        }

        // Load the stream through the player.
        let stream = Stream {
            channel_id: String::new(),
            feed_id: String::new(),
            url: url,
            quality: String::new(),
            referrer: String::new(),
            title: String::new(),
            user_agent: String::new(),
        };
        self.player.load(stream)?;
        self.player.play()?;

        Ok(())
    }
}

impl PlaybackController for IpTvPlaybackController {
    fn play(&self, channel_id: &str) {
        // Resolve candidate streams.
        let streams = self.stream_resolver.get_candidate_streams(channel_id);
        if streams.is_empty() {
            *self.current_state.write() = PlaybackState::Failed {
                channel_id: channel_id.to_string(),
                error: "no streams found for channel".to_string(),
            };
            *self.last_error.write() = Some("no streams found for channel".to_string());
            return;
        }

        let urls: Vec<String> = streams.into_iter().map(|s| s.url).collect();
        *self.candidates.write() = urls.clone();

        *self.current_state.write() = PlaybackState::Loading {
            channel_id: channel_id.to_string(),
            attempt: 1,
        };

        // Try candidates until one loads successfully.
        while let Err(e) = self.try_next_candidate() {
            // No more candidates to try.
            if self.candidates.read().is_empty() {
                *self.current_state.write() = PlaybackState::Failed {
                    channel_id: channel_id.to_string(),
                    error: e.to_string(),
                };
                *self.last_error.write() = Some(e.to_string());
                return;
            }
            // Increment attempt and try the next candidate.
            let next_attempt = {
                let state = self.current_state.read().clone();
                match state {
                    PlaybackState::Loading { attempt, .. }
                    | PlaybackState::Retrying { attempt, .. } => attempt + 1,
                    _ => 1,
                }
            };
            *self.current_state.write() = PlaybackState::Loading {
                channel_id: channel_id.to_string(),
                attempt: next_attempt,
            };
        }
    }

    fn pause(&self) {
        let _ = self.player.pause();
    }

    fn stop(&self) {
        let _ = self.player.stop();
        *self.current_state.write() = PlaybackState::Stopped;
    }

    fn seek(&self, seconds: u32) {
        let _ = self.player.seek_position(seconds);
    }
}

impl PlaybackListener for IpTvPlaybackController {
    fn on_playback_started(&self) -> Res<()> {
        let state = self.current_state.read().clone();
        match state {
            PlaybackState::Loading {
                channel_id,
                attempt: _,
            } => {
                // Get the most recently attempted URL to use as stream_id.
                let stream_id = self
                    .attempted_streams
                    .read()
                    .last()
                    .cloned()
                    .unwrap_or_default();
                *self.current_state.write() = PlaybackState::Playing {
                    channel_id,
                    stream_id,
                };
                Ok(())
            }
            PlaybackState::Retrying { .. } => {
                // We succeeded after a retry — transition to Playing.
                let channel_id = String::new();
                let stream_id = self
                    .attempted_streams
                    .read()
                    .last()
                    .cloned()
                    .unwrap_or_default();
                *self.current_state.write() = PlaybackState::Playing {
                    channel_id,
                    stream_id,
                };
                // Clear last_error since playback succeeded.
                *self.last_error.write() = None;
                Ok(())
            }
            _ => {
                // Unexpected state — ignore or log.
                Ok(())
            }
        }
    }

    fn on_playback_failed(&self, error: &str) -> Res<()> {
        // Store the error.
        *self.last_error.write() = Some(error.to_string());

        let state = self.current_state.read().clone();
        match state {
            PlaybackState::Loading {
                channel_id,
                attempt,
            }
            | PlaybackState::Retrying {
                channel_id,
                attempt,
                ..
            } => {
                // Try the next candidate stream.
                match self.try_next_candidate() {
                    Ok(()) => {
                        *self.current_state.write() = PlaybackState::Retrying {
                            channel_id,
                            attempt: attempt + 1,
                            last_error: error.to_string(),
                        };
                        Ok(())
                    }
                    Err(_) => {
                        // No more candidates — fail permanently.
                        *self.current_state.write() = PlaybackState::Failed {
                            channel_id,
                            error: error.to_string(),
                        };
                        Ok(())
                    }
                }
            }
            PlaybackState::Playing { channel_id, .. } => {
                // Playback failed mid-stream — try next candidate if available.
                match self.try_next_candidate() {
                    Ok(()) => {
                        *self.current_state.write() = PlaybackState::Retrying {
                            channel_id,
                            attempt: 1,
                            last_error: error.to_string(),
                        };
                        Ok(())
                    }
                    Err(_) => {
                        *self.current_state.write() = PlaybackState::Failed {
                            channel_id,
                            error: error.to_string(),
                        };
                        Ok(())
                    }
                }
            }
            _ => {
                // Idle or Stopped state — just record the error.
                Ok(())
            }
        }
    }

    fn on_playback_stopped(&self) -> Res<()> {
        *self.current_state.write() = PlaybackState::Stopped;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Streams;
    use std::sync::Arc;

    mockall::mock! {
        pub PlayerCtrl {}
        impl PlayerController for PlayerCtrl {
            fn load(&self, stream: Stream) -> Res<()>;
            fn play(&self) -> Res<()>;
            fn pause(&self) -> Res<()>;
            fn stop(&self) -> Res<()>;
            fn seek_position(&self, position: u32) -> Res<()>;
        }
        impl std::fmt::Debug for PlayerCtrl {
            fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
                write!(f, "MockPlayerCtrl")
            }
        }
    }

    mockall::mock! {
        pub StreamRslv {}
        impl StreamResolver for StreamRslv {
            fn get_candidate_streams(&self, channel_id: &str) -> Streams;
        }
        impl std::fmt::Debug for StreamRslv {
            fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
                write!(f, "MockStreamRslv")
            }
        }
    }

    // ── Helper ─────────────────────────────────────────────────────────

    fn make_controller(resolver: MockStreamRslv, player: MockPlayerCtrl) -> IpTvPlaybackController {
        IpTvPlaybackController::new(Arc::new(resolver), Arc::new(player))
    }

    fn make_streams(channel_id: &str, urls: Vec<&str>) -> Streams {
        urls.into_iter()
            .map(|url| Stream {
                channel_id: channel_id.to_string(),
                feed_id: String::new(),
                url: url.to_string(),
                quality: String::new(),
                referrer: String::new(),
                title: String::new(),
                user_agent: String::new(),
            })
            .collect()
    }

    // ── Tests ──────────────────────────────────────────────────────────

    #[test]
    fn initial_state_is_idle() {
        let resolver = MockStreamRslv::new();
        let player = MockPlayerCtrl::new();
        let ctrl = make_controller(resolver, player);

        assert_eq!(*ctrl.current_state.read(), PlaybackState::Idle);
        assert_eq!(*ctrl.last_error.read(), None);
        assert!(ctrl.attempted_streams.read().is_empty());
    }

    #[test]
    fn play_transitions_to_loading_and_loads_first_candidate() {
        let mut resolver = MockStreamRslv::new();
        resolver
            .expect_get_candidate_streams()
            .with(mockall::predicate::eq("ch1"))
            .times(1)
            .returning(|_| make_streams("ch1", vec!["https://example.com/stream"]));

        let mut player = MockPlayerCtrl::new();
        player.expect_load().times(1).returning(|_| Ok(()));
        player.expect_play().times(1).returning(|| Ok(()));

        let ctrl = make_controller(resolver, player);

        ctrl.play("ch1");

        assert_eq!(
            *ctrl.current_state.read(),
            PlaybackState::Loading {
                channel_id: "ch1".to_string(),
                attempt: 1,
            }
        );
        assert_eq!(ctrl.attempted_streams.read().len(), 1);
    }

    #[test]
    fn play_with_no_candidates_transitions_to_failed() {
        let mut resolver = MockStreamRslv::new();
        resolver
            .expect_get_candidate_streams()
            .with(mockall::predicate::eq("ch1"))
            .times(1)
            .returning(|_| vec![]);

        let player = MockPlayerCtrl::new();
        let ctrl = make_controller(resolver, player);

        ctrl.play("ch1");

        assert_eq!(
            *ctrl.current_state.read(),
            PlaybackState::Failed {
                channel_id: "ch1".to_string(),
                error: "no streams found for channel".to_string(),
            }
        );
        assert_eq!(
            ctrl.last_error.read().as_deref(),
            Some("no streams found for channel")
        );
    }

    #[test]
    fn on_playback_started_transitions_loading_to_playing() {
        let resolver = MockStreamRslv::new();
        let player = MockPlayerCtrl::new();
        let ctrl = make_controller(resolver, player);

        *ctrl.current_state.write() = PlaybackState::Loading {
            channel_id: "ch1".to_string(),
            attempt: 1,
        };
        ctrl.attempted_streams
            .write()
            .push("https://example.com/stream".to_string());

        // Simulate callback from a separate thread.
        let handle = std::thread::spawn(move || {
            ctrl.on_playback_started()
                .expect("on_playback_started failed");
            ctrl
        });
        let ctrl = handle.join().expect("thread panicked");

        assert_eq!(
            *ctrl.current_state.read(),
            PlaybackState::Playing {
                channel_id: "ch1".to_string(),
                stream_id: "https://example.com/stream".to_string(),
            }
        );
        assert_eq!(*ctrl.last_error.read(), None);
    }

    #[test]
    fn on_playback_failed_from_separate_thread_triggers_retry() {
        let mut resolver = MockStreamRslv::new();
        resolver
            .expect_get_candidate_streams()
            .with(mockall::predicate::eq("ch1"))
            .times(1)
            .returning(|_| {
                make_streams(
                    "ch1",
                    vec!["https://fail1.com/stream", "https://fail2.com/stream"],
                )
            });

        let mut player = MockPlayerCtrl::new();
        player.expect_load().times(2).returning(|_| Ok(()));
        player.expect_play().times(2).returning(|| Ok(()));

        let ctrl = make_controller(resolver, player);

        // Initiate play — this sets state to Loading and tries the first candidate.
        ctrl.play("ch1");
        assert_eq!(
            *ctrl.current_state.read(),
            PlaybackState::Loading {
                channel_id: "ch1".to_string(),
                attempt: 1,
            }
        );
        assert_eq!(ctrl.attempted_streams.read().len(), 1);

        // Simulate on_playback_failed from a separate thread.
        let handle = std::thread::spawn(move || {
            ctrl.on_playback_failed("connection timeout")
                .expect("on_playback_failed failed");
            ctrl
        });
        let ctrl = handle.join().expect("thread panicked");

        assert_eq!(
            *ctrl.current_state.read(),
            PlaybackState::Retrying {
                channel_id: "ch1".to_string(),
                attempt: 2,
                last_error: "connection timeout".to_string(),
            }
        );
        assert_eq!(
            ctrl.last_error.read().as_deref(),
            Some("connection timeout")
        );
        assert_eq!(ctrl.attempted_streams.read().len(), 2);
    }

    #[test]
    fn on_playback_failed_with_no_more_candidates_transitions_to_failed() {
        let mut resolver = MockStreamRslv::new();
        resolver
            .expect_get_candidate_streams()
            .with(mockall::predicate::eq("ch1"))
            .times(1)
            .returning(|_| make_streams("ch1", vec!["https://example.com/stream"]));

        let mut player = MockPlayerCtrl::new();
        player.expect_load().times(1).returning(|_| Ok(()));
        player.expect_play().times(1).returning(|| Ok(()));

        let ctrl = make_controller(resolver, player);

        ctrl.play("ch1");
        assert_eq!(ctrl.attempted_streams.read().len(), 1);

        // Failure from a separate thread — no more candidates to retry.
        let handle = std::thread::spawn(move || {
            ctrl.on_playback_failed("stream not found")
                .expect("on_playback_failed failed");
            ctrl
        });
        let ctrl = handle.join().expect("thread panicked");

        assert_eq!(
            *ctrl.current_state.read(),
            PlaybackState::Failed {
                channel_id: "ch1".to_string(),
                error: "stream not found".to_string(),
            }
        );
        assert_eq!(ctrl.last_error.read().as_deref(), Some("stream not found"));
        assert_eq!(ctrl.attempted_streams.read().len(), 1);
    }

    #[test]
    fn stop_transitions_to_stopped() {
        let mut resolver = MockStreamRslv::new();
        resolver
            .expect_get_candidate_streams()
            .with(mockall::predicate::eq("ch1"))
            .times(1)
            .returning(|_| make_streams("ch1", vec!["https://example.com/stream"]));

        let mut player = MockPlayerCtrl::new();
        player.expect_load().times(1).returning(|_| Ok(()));
        player.expect_play().times(1).returning(|| Ok(()));
        player.expect_stop().times(1).returning(|| Ok(()));

        let ctrl = make_controller(resolver, player);
        ctrl.play("ch1");
        ctrl.stop();

        assert_eq!(*ctrl.current_state.read(), PlaybackState::Stopped);
    }

    #[test]
    fn retry_failure_preserves_last_error_and_attempted_streams() {
        let mut resolver = MockStreamRslv::new();
        resolver
            .expect_get_candidate_streams()
            .with(mockall::predicate::eq("ch1"))
            .times(1)
            .returning(|_| {
                make_streams(
                    "ch1",
                    vec!["https://fail1.com/stream", "https://fail2.com/stream"],
                )
            });

        let mut player = MockPlayerCtrl::new();
        // First load succeeds (first candidate plays).
        // Second load fails (retry tries second candidate, which fails).
        player
            .expect_load()
            .with(mockall::predicate::always())
            .times(2)
            .returning(|_| {
                // static counter via thread_local won't work across calls,
                // but mockall allows per-call logic via .returning with a closure
                // that captures state
                static mut CALL_COUNT: usize = 0;
                unsafe {
                    CALL_COUNT += 1;
                    if CALL_COUNT == 1 {
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!("mock load error for candidate 2"))
                    }
                }
            });
        player.expect_play().times(1).returning(|| Ok(()));

        let ctrl = make_controller(resolver, player);

        ctrl.play("ch1");
        assert_eq!(ctrl.attempted_streams.read().len(), 1);

        // Failure from a separate thread — retry tries candidate 2.
        let handle = std::thread::spawn(move || {
            ctrl.on_playback_failed("error: stream 1 playback failed")
                .expect("on_playback_failed failed");
            ctrl
        });
        let ctrl = handle.join().expect("thread panicked");

        // Both candidates attempted.
        assert_eq!(ctrl.attempted_streams.read().len(), 2);
        // Final state is Failed since the second candidate couldn't load.
        assert!(matches!(
            *ctrl.current_state.read(),
            PlaybackState::Failed { .. }
        ));
        assert_eq!(
            ctrl.last_error.read().as_deref(),
            Some("error: stream 1 playback failed")
        );
    }
}
