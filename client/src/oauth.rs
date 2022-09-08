use std::sync::{Arc, Mutex};

use tokio::{sync::oneshot, time::Duration};
use warp::Filter;

use crate::http::DurableClient;

const WAIT_FOR_SECS: u64 = 3;

pub async fn flow(
    config: OAuthConfiguration,
    client: &DurableClient,
) -> Result<SharedAccessToken, Box<dyn std::error::Error>> {
    let (killtx, killrx) = oneshot::channel::<u8>();
    let access_code_filter = warp::get()
        .and(warp::path("redirect"))
        .and(with_config(config.clone()))
        .and(warp::query::<AccessCode>())
        .map(|c: OAuthConfiguration, ac: AccessCode| {
            c.set_access_code(&ac.code);
            warp::reply::html(format!(
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
                      <script>
                        setTimeout(() => window.location.reload(), {})
                      </script>
                    </html>
                    "#,
                ac.code,
                WAIT_FOR_SECS * 1_000 + 1_000
            ))
        });

    let auth_url = &config.get_authorize_url();
    webbrowser::open(auth_url).expect("Could not open browser");

    let port = config.get_port();
    let (_, server) = warp::serve(access_code_filter).bind_with_graceful_shutdown(
        ([127, 0, 0, 1], port),
        async move {
            println!("waiting for signal");
            killrx.await.expect("Error handling shutdown receiver");
            println!("got signal");
        },
    );
    tokio::spawn(async {
        println!("{} seconds to go...", WAIT_FOR_SECS);
        tokio::time::sleep(tokio::time::Duration::from_secs(WAIT_FOR_SECS)).await;
        killtx.send(1).expect("Could not send kill signal!");
        println!("Graceful killshot transmitted")
    });
    tokio::task::spawn(server).await?;

    let token_url = config.get_token_url();
    let body = config.to_token_request_body();
    let token = client
        .post(token_url)
        .form(&body)
        .send()
        .await?
        .json::<AccessToken>()
        .await?;

    println!("T: {:#?}", token);
    Ok(SharedAccessToken::new(token))
}

fn with_config(
    config: OAuthConfiguration,
) -> impl Filter<Extract = (OAuthConfiguration,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || config.clone())
}

#[derive(Clone)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccessToken {
    token_type: String,
    scope: String,
    expires_in: u64,
    ext_expires_in: u64,
    pub access_token: String,
    refresh_token: String,
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

    pub fn autorefresh(
        &self,
        client: DurableClient,
        token: SharedAccessToken,
        config: OAuthConfiguration,
        pad_secs: u64,
    ) {
        tokio::spawn(async move {
            loop {
                let wait_time = token.get_expires_in() - pad_secs;
                println!("{} - {} = {}", token.get_expires_in(), pad_secs, wait_time);
                println!("Refresh sleeping {} seconds...", wait_time);
                tokio::time::sleep(Duration::from_secs(wait_time)).await;
                Self::do_refresh(client.clone(), &token, &config)
                    .await
                    .expect("Could not refresh token!");
            }
        });
    }

    async fn do_refresh(
        client: DurableClient,
        token: &SharedAccessToken,
        config: &OAuthConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let refresh_url = config.get_token_url();
        let body = config.to_token_refresh_body(&token.get_refresh_token());
        let res = client
            .post(refresh_url)
            .form(&body)
            .send()
            .await?
            .json::<AccessToken>()
            .await?;
        println!("Refresh response: {:#?}", res);
        token.apply_refresh(res);
        Ok(())
    }

    fn apply_refresh(&self, payload: AccessToken) {
        let mut token = self.data.lock().unwrap();
        token.access_token = payload.access_token;
        token.expires_in = payload.expires_in;
        token.ext_expires_in = payload.ext_expires_in;
        token.refresh_token = payload.refresh_token;
        token.scope = payload.scope;
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
