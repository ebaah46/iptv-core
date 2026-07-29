use crate::domain::stream::Stream;
use anyhow::Result as Res;

/**
* Describes the way in which the library receives commands
* to stream a video. It provides the API that allows the user
* to load, play pause, and stop a video stream. Note that
* whole business logic of streaming is handled by the concrete type
* that implements this trait.
*/
pub trait PlayerController {
    // Loads a video for streaming to begin.
    fn load(&self, stream: Stream) -> Res<()>;

    // Start streaming the video
    fn play(&self) -> Res<()>;

    // Pause the playing video
    fn pause(&self) -> Res<()>;

    // Stop the playing video
    fn stop(&self) -> Res<()>;

    // Seek the playing video to a previous location
    fn seek_position(&self, position: u32) -> Res<()>;
}

/**
* Describes the way in which the library receives information
* regarding a playback video to stream a video. It allows the player controller
* to react to playback errors and react accordingly.
* The UI layer notifies the core layer with these events that
* tell if video playback is ongoing successfully or not.
*/

pub trait PlaybackListener {
    // Video streaming has started notification
    fn on_playback_started(&self) -> Res<()>;

    // Failure occurred while streaming video
    // I expect there to be some kind of error here
    // but I do not know those details so there will be a
    // TODO: add error parameter to trait
    fn on_playback_failed(&self) -> Res<()>;

    // Video streaming ended
    fn on_playback_stopped(&self) -> Res<()>;
}
