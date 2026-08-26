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

    fn seek(&self, seconds: f32);
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
    candidates: Vec<String>,
    attempted_streams: RwLock<Vec<String>>,
    current_state: RwLock<PlaybackState>,
    last_error: Option<String>,
}

impl IpTvPlaybackController {
    pub fn new(
        stream_resolver: Arc<dyn StreamResolver>,
        player: Arc<dyn PlayerController>,
    ) -> Self {
        Self {
            stream_resolver,
            player,
            candidates: vec![],
            attempted_streams: Default::default(),
            current_state: Default::default(),
            last_error: None,
        }
    }
}

impl PlaybackController for IpTvPlaybackController {
    fn play(&self, channel_id: &str) {
        todo!()
    }

    fn pause(&self) {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    fn seek(&self, seconds: f32) {
        todo!()
    }
}

impl PlaybackListener for IpTvPlaybackController {
    fn on_playback_started(&self) -> Res<()> {
        todo!()
    }

    fn on_playback_failed(&self) -> Res<()> {
        todo!()
    }

    fn on_playback_stopped(&self) -> Res<()> {
        todo!()
    }
}
