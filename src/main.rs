use std::{path::PathBuf, time::Duration};

use config::Config;
use log::LevelFilter;
use simplelog::TermLogger;
use spotify::{Client, FileTokenStorage};

mod config;
mod spotify;
mod telegram;

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

    let config = Config::load_or_create(PathBuf::from("config.json"));
    if config.api_id == 0 || config.api_hash.is_empty() {
        log::error!("looks like the config is not set up");
        return;
    }

    let tg = telegram::create_client(&config)
        .await
        .expect("failed to create telegram client");
    telegram::authorize(&tg).await.expect("failed to authorize");

    let storage = FileTokenStorage::load_or_create(PathBuf::from("token.json")).expect("failed to load token storage");

    let mut spotify = Client::new(
        env!("SPOTIFY_CLIENT_ID").to_string(),
        env!("SPOTIFY_CLIENT_SECRET").to_string(),
        storage,
    );
    spotify.authorize().await.expect("failed to authorize");

    let mut interval = tokio::time::interval(Duration::from_secs(config.interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut last_bio = String::new();
    loop {
        interval.tick().await;

        let track = match spotify.get_current_track().await {
            Ok(track) => track,
            Err(e) => {
                log::error!("failed to get current track: {e}");
                continue;
            }
        };

        if let Some(track) = track {
            log::info!("current track: {track:?}");

            fn format_duration(duration: Duration) -> String {
                let total_seconds = duration.as_secs();
                let minutes = total_seconds / 60;
                let seconds = total_seconds % 60;
                format!("{minutes}:{seconds:02}")
            }

            let bio = config
                .bio_template
                .replace("{artist}", &track.artists.join(", "))
                .replace("{title}", &track.title)
                .replace("{progress}", &format_duration(track.progress))
                .replace("{duration}", &format_duration(track.duration));

            if bio == last_bio {
                log::info!("bio is the same as last time, skipping update");
                continue;
            }

            match telegram::update_bio(&tg, bio.clone()).await {
                Ok(_) => {
                    last_bio = bio;
                    log::info!("bio updated successfully")
                }
                Err(e) => log::error!("failed to update bio: {e}"),
            }
        }
    }
}
