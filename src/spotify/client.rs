use std::time::{Duration, SystemTime};

use anyhow::Context;
use reqwest::{StatusCode, Url};
use serde::Deserialize;

use super::{Token, TokenStorage};

pub struct Client<T: TokenStorage> {
    client: reqwest::Client,
    client_id: String,
    client_secret: String,
    token_storage: T,
}

#[derive(Debug)]
pub struct Track {
    pub artists: Vec<String>,
    pub title: String,
    pub is_playing: bool,
    pub progress: Duration,
    pub duration: Duration,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    expires_in: u64,
}

impl From<TokenResponse> for Token {
    fn from(val: TokenResponse) -> Self {
        Self {
            access_token: val.access_token,
            refresh_token: val.refresh_token,
            expires: SystemTime::now() + Duration::from_secs(val.expires_in),
        }
    }
}

impl<T: TokenStorage> Client<T> {
    const REDIRECT_URI: &str = "http://localhost:3000";
    const SCOPE: &str = "user-read-currently-playing";

    pub fn new(client_id: String, client_secret: String, token_storage: T) -> Self {
        Self {
            client: reqwest::Client::new(),
            client_id,
            client_secret,
            token_storage,
        }
    }

    fn authorize_url(&self) -> String {
        const AUTHORIZE_URL: &str = "https://accounts.spotify.com/authorize";

        let mut url = Url::parse(AUTHORIZE_URL).expect("error parsing authorize url");
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("response_type", "code")
            .append_pair("scope", Self::SCOPE)
            .append_pair("redirect_uri", Self::REDIRECT_URI);

        url.to_string()
    }

    fn is_token_set(&self) -> bool {
        self.token_storage.get().is_some()
    }

    async fn get_alive_token(&mut self) -> anyhow::Result<String> {
        let token = self.token_storage.get().context("no token")?;
        if token.expires < SystemTime::now() {
            log::info!("token expired, refreshing");
            let refresh_token = token.refresh_token;
            let mut token = self
                .refresh_token(&refresh_token)
                .await
                .context("error refreshing token")?;

            token.refresh_token = refresh_token;
            self.token_storage.update(token);
        }
        Ok(token.access_token)
    }

    async fn get_token(&self, code: &str) -> anyhow::Result<Token> {
        const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";

        let response = self
            .client
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", Self::REDIRECT_URI),
            ])
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .send()
            .await
            .context("error doing request")?;

        let response: TokenResponse = response.json().await.context("error parsing token response")?;
        Ok(response.into())
    }

    async fn refresh_token(&self, refresh_token: &str) -> anyhow::Result<Token> {
        const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";

        let response = self
            .client
            .post(TOKEN_URL)
            .form(&[("grant_type", "refresh_token"), ("refresh_token", refresh_token)])
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .send()
            .await
            .context("error doing request")?;

        let response: TokenResponse = response.json().await.context("error parsing token response")?;
        Ok(response.into())
    }

    pub async fn authorize(&mut self) -> anyhow::Result<()> {
        if !self.is_token_set() {
            log::info!("please go to this URL and authorize:\n{}", self.authorize_url());
            let code = get_callback_code().context("error getting callback code")?;
            let token = self.get_token(&code).await.context("error getting token")?;
            self.token_storage.update(token);
        }
        Ok(())
    }

    pub async fn get_current_track(&mut self) -> anyhow::Result<Option<Track>> {
        const CURRENT_TRACK_URL: &str = "https://api.spotify.com/v1/me/player/currently-playing";

        #[derive(Deserialize)]
        struct Response {
            item: Item,
            progress_ms: u64,
            is_playing: bool,
        }

        #[derive(Deserialize)]
        struct Item {
            name: String,
            artists: Vec<Artist>,
            duration_ms: u64,
        }

        #[derive(Deserialize)]
        struct Artist {
            name: String,
        }

        let response = self
            .client
            .get(CURRENT_TRACK_URL)
            .header("Authorization", format!("Bearer {}", self.get_alive_token().await?))
            .send()
            .await?;

        if response.status() == StatusCode::NO_CONTENT {
            return Ok(None);
        }

        let response: Response = response.json().await?;
        Ok(Some(Track {
            artists: response.item.artists.iter().map(|artist| artist.name.clone()).collect(),
            title: response.item.name,
            is_playing: response.is_playing,
            duration: Duration::from_millis(response.item.duration_ms),
            progress: Duration::from_millis(response.progress_ms),
        }))
    }
}

fn get_callback_code() -> Option<String> {
    let server = tiny_http::Server::http("127.0.0.1:3000").expect("error creating server");

    for request in server.incoming_requests() {
        let url = request.url();
        let mut params = form_urlencoded::parse(url.split('?').nth(1).unwrap_or("").as_bytes());

        if let Some(code) = params
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.to_string())
        {
            const RESPONSE_STRING: &str = "ok, continue in the application";
            request
                .respond(tiny_http::Response::from_string(RESPONSE_STRING).with_status_code(200))
                .ok();
            return Some(code.to_string());
        }
    }

    None
}
