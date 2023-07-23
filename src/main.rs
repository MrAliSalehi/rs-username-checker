use grammers_client::{Client, Config, InitParams, SignInError};
use grammers_session::Session;
use grammers_tl_types as tl;
use std::{env::var, sync::Arc, error::Error, io::Write};
use std::time::Duration;
use clap::Parser;
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub type MyResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[tokio::main]
async fn main() -> MyResult<()> {
    dotenv::dotenv().ok();
    let config = AppCommands::parse();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap())
        .with(tracing_subscriber::fmt::layer())
        .init();
    let (api_hash, api_id, session_file) = (
        Arc::new(var("API_HASH")?),
        var("API_ID")?.parse::<i32>()?,
        var("SESSION_FILE")?,
    );

    let client = Client::connect(Config {
        api_hash: api_hash.to_string(),
        api_id,
        params: InitParams {
            catch_up: true,
            ..Default::default()
        },
        session: Session::load_file_or_create(&session_file)?,
    }).await?;

    if !client.is_authorized().await? {
        let (login_token, code) = (
            client
                .request_login_code(&var("PHONE")?, api_id, &api_hash)
                .await?,
            prompt("verification code: ").await?,
        );

        if let Err(e) = client.sign_in(&login_token, &code).await {
            let SignInError::PasswordRequired(password_token) = e else {
                return Err(e.into());
            };

            let password = prompt(&format!(
                "2FA password (hint: {}): ",
                password_token.hint().unwrap_or_default()
            )).await?;
            client.check_password(password_token, password).await?;
        };
        client.session().save_to_file(&session_file)?
    }

    let me = client.get_me().await?;
    tracing::info!("logged in as {} (ID: {})\nchecking: {}",me.username().unwrap_or_default(),me.id(),&config.username);

    loop {
        sleep(Duration::from_secs(1)).await;

        let result = client.invoke(&tl::functions::account::CheckUsername { username: config.username.to_owned() }).await;
        match result {
            Ok(is_available) => {
                if is_available {
                    let user = client.invoke(&tl::functions::account::UpdateUsername { username: config.username.to_owned() }).await;
                    match user {
                        Ok(_) => {
                            tracing::trace!("username registered successfully!");
                            break;
                        }
                        Err(err) => {
                            if err.is("USERNAME_PURCHASE_AVAILABLE") {
                                tracing::warn!("username is available only in fragment.com");
                                break;
                            } else if err.is("USERNAME_INVALID") {
                                tracing::warn!("provided username is invalid");
                                break;
                            } else if err.is("USERNAME_NOT_MODIFIED") {
                                tracing::warn!("username is already registered for this account");
                                break;
                            } else if err.is("USERNAME_OCCUPIED") {
                                continue;
                            } else {
                                tracing::warn!("{}",err);
                                break;
                            }
                        }
                    }
                }
            }

            Err(err) => {
                if err.is("USERNAME_PURCHASE_AVAILABLE") {
                    tracing::warn!("username is available only in fragment.com");
                    break;
                } else if err.is("USERNAME_INVALID"){
                    tracing::warn!("provided username is invalid");
                    break;
                }else if !err.is("USERNAME_INVALID"){
                    tracing::warn!("{}",err);
                }
            }
        }

    }

    tracing::trace!("operation finished");

    Ok(())
}


pub async fn prompt(message: &str) -> MyResult<String> {
    let mut stdout = std::io::stdout();
    stdout.write_all(message.as_bytes())?;
    stdout.flush()?;

    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;

    Ok(line.trim().to_string())
}


#[derive(Parser)]
#[clap(version)]
pub struct AppCommands {
    #[clap(index = 1, help = "username to check without @")]
    pub username: String,

}