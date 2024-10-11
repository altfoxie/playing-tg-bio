use std::time::Duration;

use osakit::declare_script;

use crate::UnifiedTrack;

#[derive(Debug)]
pub struct Track {
    pub player_state: PlayerState,
    pub artist: String,
    pub title: String,
    pub progress: Duration,
    pub duration: Duration,
}

impl From<Track> for UnifiedTrack {
    fn from(val: Track) -> Self {
        UnifiedTrack {
            artist: val.artist,
            title: val.title,
            progress: val.progress,
            duration: val.duration,
        }
    }
}

#[derive(Debug)]
pub enum PlayerState {
    Playing,
    Paused,
    Stopped,
}

impl From<&str> for PlayerState {
    fn from(s: &str) -> Self {
        match s {
            "playing" => Self::Playing,
            "paused" => Self::Paused,
            "stopped" => Self::Stopped,
            _ => unreachable!(),
        }
    }
}

declare_script! {
  #[language(AppleScript)]
  #[source("
    on get_info()
      if application \"Music\" is running then
       	tell application \"Music\"
        		set a to \"\"
        		set n to \"\"
        		set p to 0
        		set d to 0
       			try
        				set tr to current track
        				set a to artist of tr
        				set n to name of tr
        				set p to player position
        				set d to duration of tr
       			end try
        		return {player state as string, a, n, p, d}
       	end tell
      end if
    end get_track_info
  ")]
  pub Script {
    fn get_info() -> (String, String, String, f64, f64);
  }
}

// плевать
unsafe impl Send for Script {}
unsafe impl Sync for Script {}

impl Script {
    pub fn get_current_track(&self) -> Result<Track, osakit::ScriptFunctionRunError> {
        let (state, artist, title, progress, duration) = self.get_info()?;
        Ok(Track {
            player_state: state.as_str().into(),
            artist,
            title,
            progress: Duration::from_secs_f64(progress),
            duration: Duration::from_secs_f64(duration),
        })
    }
}
