use std::{io::BufRead, sync::LazyLock, time::Duration};

use anyhow::Context;
use async_trait::async_trait;
use grammers_client::{Client, FixedReconnect, ReconnectionPolicy, SignInError};
use grammers_session::Session;
use grammers_tl_types::functions::account::UpdateProfile;
use serde::Deserialize;
use serde_json::json;

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

static RECONNECTION_POLICY: &'static dyn ReconnectionPolicy = &FixedReconnect {
    attempts: 10,
    delay: Duration::from_secs(1),
};

pub async fn create_client(api_id: i32, api_hash: String) -> anyhow::Result<Client> {
    Ok(Client::connect(grammers_client::Config {
        session: Session::load_file_or_create("session.bin")?,
        api_id,
        api_hash,
        params: grammers_client::InitParams {
            reconnection_policy: RECONNECTION_POLICY,
            ..Default::default()
        },
    })
    .await?)
}

fn prompt(name: String) -> anyhow::Result<String> {
    log::info!("enter {}: ", name);
    let stdin = std::io::stdin().lock();
    let input = stdin.lines().next().context("no input")??;
    Ok(input.trim().to_string())
}

pub async fn authorize(client: &Client) -> anyhow::Result<()> {
    if client.is_authorized().await? {
        log::info!("already authorized");
        return Ok(());
    }

    log::info!("not authorized");

    let phone = prompt("phone number".to_string())?;
    let token = client.request_login_code(&phone).await?;
    let code = prompt("code".to_string())?;
    let mut signed_in = client.sign_in(&token, &code).await;

    if let Err(SignInError::PasswordRequired(password_token)) = signed_in {
        let hint = password_token.hint().unwrap_or_default();
        let password = prompt(if hint.is_empty() {
            "password".to_string()
        } else {
            format!("password ({})", hint)
        })?;

        signed_in = client.check_password(password_token, &password).await;
    }

    match signed_in {
        Ok(user) => log::info!("authorized as {}", user.full_name()),
        Err(e) => log::error!("failed to authorize: {}", e),
    }

    client.session().save_to_file("session.bin")?;

    Ok(())
}

pub async fn update_bio(client: &Client, bio: String) -> anyhow::Result<()> {
    client
        .invoke(&UpdateProfile {
            about: Some(bio),
            first_name: None,
            last_name: None,
        })
        .await?;

    Ok(())
}

pub async fn update_channel_message(
    token: String,
    channel_id: i64,
    message_id: i64,
    text: String,
) -> anyhow::Result<()> {
    #[derive(Deserialize)]
    struct Response {
        ok: bool,
    }

    let response = CLIENT
        .post(format!("https://api.telegram.org/bot{}/editMessageText", token))
        .json(&json!({
            "chat_id": channel_id,
            "message_id": message_id,
            "text": text,
        }))
        .send()
        .await?
        .json::<Response>()
        .await?;

    if !response.ok {
        anyhow::bail!("not ok");
    }
    Ok(())
}

#[async_trait]
pub trait Updater {
    async fn update(&self, text: String) -> anyhow::Result<()>;
}

pub struct BioUpdater(pub Client);

#[async_trait]
impl Updater for BioUpdater {
    async fn update(&self, text: String) -> anyhow::Result<()> {
        update_bio(&self.0, text).await
    }
}

pub struct ChannelUpdater {
    pub token: String,
    pub channel_id: i64,
    pub message_id: i64,
}

#[async_trait]
impl Updater for ChannelUpdater {
    async fn update(&self, text: String) -> anyhow::Result<()> {
        update_channel_message(self.token.clone(), self.channel_id, self.message_id, text).await
    }
}
