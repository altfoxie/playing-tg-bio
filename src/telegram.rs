use std::{io::BufRead, time::Duration};

use anyhow::Context;
use grammers_client::{Client, FixedReconnect, ReconnectionPolicy, SignInError};
use grammers_session::Session;
use grammers_tl_types::functions::account::UpdateProfile;

use crate::config::Config;

static RECONNECTION_POLICY: &dyn ReconnectionPolicy = &FixedReconnect {
    attempts: 10,
    delay: Duration::from_secs(1),
};

pub async fn create_client(config: &Config) -> anyhow::Result<Client> {
    Ok(Client::connect(grammers_client::Config {
        session: Session::load_file_or_create("session.bin")?,
        api_id: config.api_id,
        api_hash: config.api_hash.to_string(),
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
