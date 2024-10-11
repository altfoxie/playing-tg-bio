use async_trait::async_trait;
use config::{Config, Service, TelegramConfig};
use log::LevelFilter;
use simplelog::TermLogger;
use spotify::{Client, FileTokenStorage};
use std::{path::PathBuf, time::Duration};
use telegram::Updater;
#[cfg(target_os = "macos")]
mod apple_music;
mod config;
mod spotify;
mod telegram;
#[derive(Debug)]
pub struct UnifiedTrack {
    artist: String,
    title: String,
    progress: Duration,
    duration: Duration,
}
#[async_trait]
pub trait TrackProvider {
    async fn get_current_track(&mut self) -> anyhow::Result<Option<UnifiedTrack>>;
}
pub struct SpotifyProvider {
    client: Client<FileTokenStorage>,
}
impl SpotifyProvider {
    pub fn new(client: Client<FileTokenStorage>) -> Self {
        Self { client }
    }
}
#[async_trait]
impl TrackProvider for SpotifyProvider {
    async fn get_current_track(&mut self) -> anyhow::Result<Option<UnifiedTrack>> {
        self.client
            .get_current_track()
            .await
            .map(|track| track.map(|track| track.into()))
    }
}
#[cfg(target_os = "macos")]
pub struct AppleMusicProvider {
    script: apple_music::Script,
}

#[cfg(target_os = "macos")]
impl Default for AppleMusicProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "macos")]
impl AppleMusicProvider {
    pub fn new() -> Self {
        Self {
            script: apple_music::Script::new().expect("error creating apple music script"),
        }
    }
}
#[cfg(target_os = "macos")]
#[async_trait]
impl TrackProvider for AppleMusicProvider {
    async fn get_current_track(&mut self) -> anyhow::Result<Option<UnifiedTrack>> {
        self.script
            .get_current_track()
            .map_err(|e| e.into())
            .map(|track| Some(track.into()))
    }
}
#[tokio::main]
async fn main() {
    TermLogger::init(
        LevelFilter::Info,
        simplelog::ConfigBuilder::new()
            .add_filter_allow_str(module_path!())
            .build(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    let config = Config::load_or_create(PathBuf::from("config.json")).expect("failed to load config");
    let mut track_provider: Box<dyn TrackProvider> = match config.service {
        Service::Spotify => {
            let mut client = spotify::Client::new(
                env!("SPOTIFY_CLIENT_ID").to_string(),
                env!("SPOTIFY_CLIENT_SECRET").to_string(),
                spotify::FileTokenStorage::load_or_create(PathBuf::from("token.json"))
                    .expect("failed to load token storage"),
            );
            client.authorize().await.expect("failed to authorize");
            Box::new(SpotifyProvider::new(client))
        }

        #[cfg(target_os = "macos")]
        Service::AppleMusic => Box::new(AppleMusicProvider::new()),
    };

    let updater: Box<dyn Updater> = match config.telegram {
        TelegramConfig::Bio { api_id, api_hash, .. } => {
            let client = telegram::create_client(api_id, api_hash)
                .await
                .expect("failed to create telegram client");
            telegram::authorize(&client).await.expect("failed to authorize");
            Box::new(telegram::BioUpdater(client))
        }

        TelegramConfig::Channel {
            token,
            channel_id,
            message_id,
            ..
        } => Box::new(telegram::ChannelUpdater {
            token,
            channel_id,
            message_id,
        }),
    };

    let mut interval = tokio::time::interval(Duration::from_secs(config.interval));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut last_text = String::new();

    loop {
        interval.tick().await;

        let track = match track_provider.get_current_track().await {
            Ok(track) => track,
            Err(e) => {
                log::error!("failed to get current track: {e}");
                continue;
            }
        };

        let text = match track {
            Some(track) => {
                log::info!("current track: {track:?}");

                fn format_duration(duration: Duration) -> String {
                    let total_seconds = duration.as_secs();
                    let minutes = total_seconds / 60;
                    let seconds = total_seconds % 60;
                    format!("{minutes}:{seconds:02}")
                }

                config
                    .template
                    .replace("{artist}", &track.artist)
                    .replace("{title}", &track.title)
                    .replace("{progress}", &format_duration(track.progress))
                    .replace("{duration}", &format_duration(track.duration))
            }

            None => config.default.clone(),
        };

        if text == last_text {
            log::info!("text is the same as last time, skipping update");
            continue;
        }

        match updater.update(text.clone()).await {
            Ok(_) => {
                last_text = text;
                log::info!("updated successfully")
            }
            Err(e) => log::error!("failed to update: {e}"),
        }
    }
}
