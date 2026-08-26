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
