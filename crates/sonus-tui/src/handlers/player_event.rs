use sonus_core::player::PlayerEvent;
use crate::app::App;
use crate::state::app_state::PlayStatus;

impl App {
    pub(crate) fn handle_player_event(&mut self, evt: PlayerEvent) {
        match evt {
            PlayerEvent::NowPlaying(title, artist) => {
                self.state.player.status = PlayStatus::Playing;
                self.state.player.current_track = Some((title, artist));
                self.update_mpris_state();
            }
            PlayerEvent::Progress(pos, dur) => {
                self.state.player.position = pos;
                self.state.player.duration = dur;
                self.update_mpris_position(pos);
            }
            PlayerEvent::Finished => {
                let last_video_id = self.state.player.current_video_id.clone();
                self.state.player.status = PlayStatus::Stopped;
                self.state.player.current_track = None;
                self.state.player.current_video_id = None;
                self.state.player.position = 0.0;
                self.state.player.duration = 0.0;
                self.cover_image = None;
                self.current_cover_video_id = None;
                self.update_mpris_state();
                self.play_next_in_queue();
                if self.state.auto_play && self.state.queue.is_empty() {
                    if let Some(ref vid) = last_video_id {
                        self.fetch_recommendations(vid);
                    }
                }
            }
            PlayerEvent::Error(e) => {
                self.state.status_message = Some(e);
            }
            PlayerEvent::StatusMessage(msg) => {
                self.state.status_message = Some(msg);
            }
        }
    }
}
