use axum::{
    extract::{Extension, Query},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::{
    env, fmt,
    net::SocketAddr,
    process::Command,
    sync::{Arc, Mutex},
};
use tokio::time::Duration;
use tower::ServiceBuilder;

use crate::http::DurableClient;

#[tracing::instrument]
pub async fn flow(
    config: OAuthConfiguration,
    client: &DurableClient,
    shutdown_after_secs: u64,
) -> Result<SharedAccessToken, Box<dyn std::error::Error>> {
    let auth_url = &config.get_authorize_url();
    if cfg!(unix) {
        // webbrowser doesn't seem to work on WSL.
        // In reality, this is not unix specific code but vitale232 WSL specific code
        let browser = env::var("BROWSER").unwrap();
        tracing::info!("BROWSER: {}", browser);
        Command::new(browser)
            .arg(&auth_url)
            .spawn()
            .expect("Could not open browser");
    } else {
        webbrowser::open(auth_url).expect("Could not open browser");
    }

    run_server(&config, shutdown_after_secs).await?;
    tracing::info!("The OAuth server has terminated. Fetching token.");

    let token_url = config.get_token_url();
    let body = config.to_token_request_body();
    let token = client
        .post(token_url)
        .form(&body)
        .send()
        .await?
        .json::<AccessToken>()
        .await?;

    tracing::info!("T: {:#?}", token);
    Ok(SharedAccessToken::new(token))
}

#[tracing::instrument]
async fn run_server(
    config: &OAuthConfiguration,
    shutdown_after_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let refresh_time = shutdown_after_secs * 1000 + 300;

    let app = Router::new()
        .route("/redirect", get(handle_oauth_redirect))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(config.clone()))
                .layer(Extension(refresh_time))
                .into_inner(),
        );

    let port = config.get_port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_timer(shutdown_after_secs))
        .await?;

    Ok(())
}

#[tracing::instrument]
async fn handle_oauth_redirect(
    config: Extension<OAuthConfiguration>,
    refresh_after: Extension<u64>,
    Query(access): Query<AccessCode>,
) -> impl IntoResponse {
    tracing::debug!("Access code received at server with value {:#?}", access);
    config.set_access_code(&access.code);
    Html(format!(
        r#"
        <!DOCTYPE html>
        <html lang='en'>
          <head>
            <meta charset='UTF-8' />
            <meta http-equiv='X-UA-Compatible' content='IE=edge' />
            <meta name='viewport' content='device-width' />
            <title>Warp OAuth</title>
          </head>
          <body>
            <h1>Access Code</h1>
            <p>Access code recieved!</p>
            <code>{}</code>
          </body>
        </html>
        <script>
          setTimeout(() => window.location.reload(), {});
        </script>
        "#,
        access.code, *refresh_after
    ))
}

#[tracing::instrument]
async fn shutdown_timer(wait_for_secs: u64) {
    tracing::debug!("Waiting for {} seconds to send shutdown...", wait_for_secs);
    tokio::time::sleep(Duration::from_secs(wait_for_secs)).await;
    tracing::info!("Shutdown signal fired!");
}

#[derive(Clone, Debug)]
pub struct OAuthConfiguration {
    data: Arc<Mutex<Config>>,
}

impl OAuthConfiguration {
    pub fn new(client_id: &str, tenant_id: &str, scope: &str) -> Self {
        OAuthConfiguration {
            data: Arc::new(Mutex::new(Config::new(client_id, tenant_id, scope))),
        }
    }

    pub fn set_access_code(&self, ac: &str) {
        let mut config = self.data.lock().unwrap();
        config.set_access_code(ac);
    }

    pub fn get_token_url(&self) -> String {
        let config = self.data.lock().unwrap();
        config.get_token_url()
    }

    fn get_authorize_url(&self) -> String {
        let config = self.data.lock().unwrap();
        config.get_authorize_url()
    }

    fn get_port(&self) -> u16 {
        let config = self.data.lock().unwrap();
        config.get_port()
    }

    fn to_token_request_body(&self) -> AccessTokenRequestBody {
        let config = self.data.lock().unwrap();
        config.to_token_request_body()
    }

    fn to_token_refresh_body(&self, refresh_token: &str) -> RefreshTokenRequestBody {
        let config = self.data.lock().unwrap();
        RefreshTokenRequestBody {
            client_id: config.get_client_id(),
            grant_type: "refresh_token".into(),
            scope: config.get_scope(),
            refresh_token: refresh_token.into(),
        }
    }
}

#[derive(Clone, Debug)]
struct Config {
    pub client_id: String,
    pub tenant_id: String,
    pub port: u16,
    pub scope: String,
    pub access_code: Option<String>,
}

impl Config {
    pub fn new(client_id: &str, tenant_id: &str, scope: &str) -> Self {
        Config {
            client_id: client_id.into(),
            tenant_id: tenant_id.into(),
            port: 42069,
            access_code: None,
            scope: scope.into(),
        }
    }

    /// Panics! Panics when invoked while the `access_code` property is the
    /// `None` variant
    fn to_token_request_body(&self) -> AccessTokenRequestBody {
        let code = match &self.access_code {
            Some(ac) => ac,
            None => panic!("No Access Code available."),
        };
        AccessTokenRequestBody {
            code: code.into(),
            client_id: self.client_id.clone(),
            redirect_uri: self.get_redirect_uri(),
            grant_type: String::from("authorization_code"),
        }
    }

    fn get_authorize_url(&self) -> String {
        format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize?{}",
            self.tenant_id,
            self.get_authorize_query()
        )
    }

    fn get_authorize_query(&self) -> String {
        format!(
            "response_type=code&client_id={}&redirect_uri={}&scope={}&sso_reload=true",
            self.client_id,
            self.get_redirect_uri(),
            self.scope
        )
    }

    fn get_token_url(&self) -> String {
        format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        )
    }

    fn get_client_id(&self) -> String {
        self.client_id.clone()
    }

    fn get_scope(&self) -> String {
        self.scope.clone()
    }

    fn get_redirect_uri(&self) -> String {
        format!("http://localhost:{}/redirect", self.port)
    }

    fn set_access_code(&mut self, ac: &str) {
        self.access_code = Some(ac.into());
    }

    fn get_port(&self) -> u16 {
        self.port
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AccessToken {
    token_type: String,
    scope: String,
    expires_in: u64,
    ext_expires_in: u64,
    pub access_token: String,
    refresh_token: String,
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessToken")
            .field("token_type", &self.token_type)
            .field("scope", &self.scope)
            .field("expires_in", &self.expires_in)
            .field("ext_expires_in", &self.ext_expires_in)
            .field("refresh_token", &"[REDACTED]")
            .field("acess_token", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct SharedAccessToken {
    data: Arc<Mutex<AccessToken>>,
}

impl SharedAccessToken {
    fn new(token: AccessToken) -> Self {
        SharedAccessToken {
            data: Arc::new(Mutex::new(token)),
        }
    }

    #[tracing::instrument]
    pub fn autorefresh(
        token: SharedAccessToken,
        client: DurableClient,
        config: OAuthConfiguration,
        pad_secs: u64,
    ) {
        tokio::spawn(async move {
            loop {
                let wait_time = token.get_expires_in() - pad_secs;
                tracing::info!(
                    "{} - {} = {}. Fresh sleeping {} seconds...",
                    token.get_expires_in(),
                    pad_secs,
                    wait_time,
                    wait_time
                );
                tokio::time::sleep(Duration::from_secs(wait_time)).await;
                tracing::info!("oauth::auto_refresh awake. Refreshing token!");
                Self::do_refresh(client.clone(), &token, &config)
                    .await
                    .expect("Could not refresh token!");
            }
        });
    }

    #[tracing::instrument]
    async fn do_refresh(
        client: DurableClient,
        token: &SharedAccessToken,
        config: &OAuthConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let refresh_url = config.get_token_url();
        let body = config.to_token_refresh_body(&token.get_refresh_token());
        tracing::debug!("Refresh token request body: {:#?}", body);
        let res = client
            .post(refresh_url)
            .form(&body)
            .send()
            .await?
            .json::<AccessToken>()
            .await?;
        tracing::info!("Refresh response: {:#?}", res);
        token.apply_refresh(res);
        Ok(())
    }

    #[tracing::instrument]
    fn apply_refresh(&self, payload: AccessToken) {
        tracing::debug!("Applying token refresh payload {:#?}", payload);
        let mut token = self.data.lock().unwrap();
        token.access_token = payload.access_token;
        token.expires_in = payload.expires_in;
        token.ext_expires_in = payload.ext_expires_in;
        token.refresh_token = payload.refresh_token;
        token.scope = payload.scope;
        tracing::debug!("New token: {:#?}", token);
    }

    fn get_expires_in(&self) -> u64 {
        let token = self.data.lock().unwrap();
        token.expires_in
    }

    fn get_refresh_token(&self) -> String {
        let token = self.data.lock().unwrap();
        token.refresh_token.clone()
    }

    pub fn get_access_token(&self) -> String {
        let token = self.data.lock().unwrap();
        token.access_token.clone()
    }
}
#[derive(Debug, Deserialize, Serialize)]
struct AccessTokenRequestBody {
    client_id: String,
    redirect_uri: String,
    code: String,
    grant_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct RefreshTokenRequestBody {
    client_id: String,
    grant_type: String,
    scope: String,
    refresh_token: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct AccessCode {
    pub code: String,
}
