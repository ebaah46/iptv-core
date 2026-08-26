//! Throwaway spike -- NOT part of iptv-core.
//!
//! Goal: validate the assumptions baked into `PlayerController` /
//! `PlaybackListener` before building `IptvPlaybackController` for real.
//!
//! Specifically answering:
//!   1. Which thread do playback events actually arrive on?
//!   2. Does `referrer` / `user_agent` actually propagate to every HTTP
//!      request an HLS stream makes (manifest + every segment), not just
//!      the first one?
//!   3. What does a real failure look like as data (structured error vs.
//!      opaque)?
//!   4. Roughly how long does load -> first frame take?
//!
//! Run against 2-3 real stream URLs: one that plays cleanly, one that's
//! known to require a referrer/user-agent to work, and one deliberately
//! broken URL to force the failure path.

use gst::prelude::*;
use gstreamer as gst;
use std::sync::Arc;
use std::time::Instant;

struct Candidate {
    url: &'static str,
    referrer: Option<&'static str>,
    user_agent: Option<&'static str>,
}

fn main() {
    gst::init().expect("failed to init GStreamer");

    // TODO: replace with real iptv-org stream URLs before running --
    // include at least one header-sensitive stream, and one broken URL.
    let candidates = vec![
        Candidate {
            url: "https://example.com/valid_stream.m3u8",
            referrer: None,
            user_agent: None,
        },
        Candidate {
            url: "https://dwamdstream102.akamaized.net/hls/live/2015525/dwstream102/index.m3u8",
            referrer: Some("https://required-referrer.example.com/"),
            user_agent: Some("Mozilla/5.0 (SpikeTest)"),
        },
        Candidate {
            url: "https://bitdash-a.akamaihd.net/content/sintel/hls/playlist.m3u8",
            referrer: Some("https://bitmovin.com/"),
            user_agent: Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)"),
        },
    ];

    for candidate in candidates {
        println!("\n=== Attempting: {} ===", candidate.url);
        run_candidate(&candidate);
    }
}

fn run_candidate(candidate: &Candidate) {
    let pipeline = Arc::new(
        gst::ElementFactory::make("playbin")
            .property("uri", candidate.url)
            .build()
            .expect("failed to create playbin"),
    );

    // playbin creates its HTTP source element internally (souphttpsrc for
    // http/https) -- `source-setup` is the hook to reach in and set headers
    // on it. Logging the thread here answers spike question 1 for the
    // *setup* path; the bus loop below answers it for playback events.
    let referrer = candidate.referrer.map(str::to_string);
    let user_agent = candidate.user_agent.map(str::to_string);

    pipeline.connect("source-setup", false, move |values| {
        let source = values[1].get::<gst::Element>().expect("source-setup arg");
        println!(
            "[source-setup] thread={:?} element={}",
            std::thread::current().id(),
            source
                .factory()
                .map(|f| f.name().to_string())
                .unwrap_or_default()
        );

        if let Some(ua) = &user_agent {
            if source.has_property("user-agent") {
                source.set_property("user-agent", ua.as_str());
            }
        }
        if let Some(referrer) = &referrer {
            if source.has_property("extra-headers") {
                let headers = gst::Structure::builder("headers")
                    .field("Referer", referrer.as_str())
                    .build();
                source.set_property("extra-headers", headers);
            }
        }
        None
    });

    let bus = pipeline.bus().expect("pipeline has no bus");
    let bus_clone = bus.clone();
    let start = Instant::now();

    pipeline
        .set_state(gst::State::Playing)
        .expect("unable to request Playing state");

    // Polling the bus like this is the simplest way to observe events for a
    // spike -- but note this itself is data: GStreamer marshals messages
    // from its internal worker threads onto whatever thread calls this.
    // A real PlaybackController will likely want a bus watch on a known
    // thread (or a channel) rather than blocking here -- confirm that
    // shape once this spike answers the basic questions.
    let pipeline_clone = pipeline.clone();
    let handle = std::thread::spawn(move || {
        for msg in bus_clone.iter_timed(gst::ClockTime::from_seconds(15)) {
            let thread_id = std::thread::current().id();
            match msg.view() {
                gst::MessageView::AsyncDone(_) => {
                    println!(
                        "[onPlaybackStarted?] thread={:?} elapsed={:?}",
                        thread_id,
                        start.elapsed()
                    );
                    break;
                }
                gst::MessageView::Error(err) => {
                    // This structure -- an error code plus a debug string -- is
                    // what spike question 3 is checking: is this enough for
                    // PlaybackListener::onPlaybackFailed(error) to act on
                    // (e.g. distinguish "not found" from "auth failed"), or is
                    // it too opaque to be useful?
                    println!(
                        "[onPlaybackFailed] thread={:?} elapsed={:?} error={:?} debug={:?}",
                        thread_id,
                        start.elapsed(),
                        err.error(),
                        err.debug()
                    );
                    break;
                }
                gst::MessageView::Eos(_) => {
                    println!("[onPlaybackStopped/EOS] thread={:?}", thread_id);
                    break;
                }
                gst::MessageView::StateChanged(state) => {
                    if state
                        .src()
                        .map(|s| s == pipeline_clone.upcast_ref::<gst::Object>())
                        .unwrap_or(false)
                    {
                        println!(
                            "[state change] thread={:?} {:?} -> {:?}",
                            thread_id,
                            state.old(),
                            state.current()
                        );
                    }
                }
                _ => {}
            }
        }
    });
    handle.join().expect("thread has panicked");
    pipeline
        .set_state(gst::State::Null)
        .expect("unable to reset pipeline to Null");
}
