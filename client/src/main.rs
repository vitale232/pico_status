#[macro_use]
extern crate dotenv_codegen;
#[macro_use]
extern crate serde;

use std::{
    fmt::format,
    sync::{Arc, Mutex},
};

use tokio::sync::oneshot;
use urlencoding::encode;
use warp::{path::param, Filter};

static CLIENT_ID: &str = dotenv!("CLIENT_ID");
static CLIENT_SECRET: &str = dotenv!("CLIENT_SECRET");
static TENANT_ID: &str = dotenv!("TENANT_ID");

#[derive(Clone, Debug)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub tenant_id: String,
    pub port: u16,
    pub access_code: Option<String>,
}

impl Config {
    fn new(client_id: &str, client_secret: &str, tenant_id: &str) -> Self {
        Config {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            tenant_id: tenant_id.into(),
            port: 42069,
            access_code: None,
        }
    }

    fn get_authorize_url(&self, scope: &str) -> String {
        format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize?{}",
            self.tenant_id,
            self.get_authorize_query(scope)
        )
    }

    fn get_authorize_query(&self, scope: &str) -> String {
        format!(
            "response_type=code&client_id={}&redirect_uri={}&scope={}&sso_reload=true",
            self.client_id,
            self.get_redirect_uri(),
            scope
        )
    }

    fn get_token_url(&self) -> String {
        format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        )
    }

    fn get_access_token_body(&self) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let access_code = match &self.access_code {
            Some(ac) => ac,
            None => panic!("No Access Code available."),
        };
        let params = vec![
            ("code".into(), access_code.into()),
            ("client_id".into(), String::from(&self.client_id)),
            ("redirect_uri".into(), self.get_redirect_uri()),
            ("grant_type".into(), "authorization_code".into()),
        ];
        Ok(params)
    }

    fn get_redirect_uri(&self) -> String {
        encode(&format!("http://localhost:{}/redirect", self.port)).into()
    }

    fn set_access_code(&mut self, ac: &str) {
        self.access_code = Some(ac.into());
    }

    fn get_port(&self) -> u16 {
        self.port
    }
}

type OAuthConfiguration = Arc<Mutex<Config>>;

#[derive(Debug, Deserialize, Serialize)]
struct AccessCode {
    pub code: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AccessToken {
    access_token: String,
    token_type: String,
    expires_in: i64,
    scope: String,
    refresh_token: String,
    user_id: String,
}

impl AccessToken {
    pub fn new(
        token_type: &str,
        expires_in: i64,
        scope: &str,
        access_token: &str,
        refresh_token: &str,
        user_id: &str,
    ) -> Self {
        AccessToken {
            token_type: token_type.into(),
            expires_in,
            scope: scope.into(),
            access_token: access_token.into(),
            refresh_token: refresh_token.into(),
            user_id: user_id.into(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (killtx, killrx) = oneshot::channel::<u8>();
    let config = Arc::new(Mutex::new(Config::new(CLIENT_ID, CLIENT_SECRET, TENANT_ID)));
    let scope = "Presence.Read Calendars.read offline_access";

    webbrowser::open(&config.lock().unwrap().get_authorize_url(&scope))
        .expect("Could not open browser");

    let access_code = warp::get()
        .and(warp::path("redirect"))
        .and(with_config(config.clone()))
        .and(warp::query::<AccessCode>())
        .map(|c: OAuthConfiguration, ac: AccessCode| {
            c.lock().unwrap().set_access_code(&ac.code);
            println!("AC:::: {:?}", ac);
            ac.code
        });

    let p = config.lock().unwrap().get_port();
    let (_, server) =
        warp::serve(access_code).bind_with_graceful_shutdown(([127, 0, 0, 1], p), async move {
            println!("waiting for signal");
            killrx.await.expect("Error handling shutdown receiver");
            println!("got signal");
        });

    tokio::spawn(async {
        let secs = 5;
        println!("Dooms day prepping, {} seconds to go...", secs);
        tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
        killtx.send(1).expect("Could not send kill signal!");
    });

    match tokio::join!(tokio::task::spawn(server)).0 {
        Ok(()) => println!("serving..."),
        Err(e) => println!("An error occurred in Join! {:?}", e),
    }

    println!("sleeping... This is where the program can start with a token?");
    println!("Speaking of tokens: {:#?}", config);
    let client = reqwest::Client::new();
    let auth_url = config.lock().unwrap().get_authorize_url(scope);
    let params = config.lock().unwrap().get_authorize_query(scope);
    let res = client.post(auth_url).form(&params).send().await?;
    println!("Response: {:#?}", res);
    tokio::time::sleep(tokio::time::Duration::from_secs(69)).await;
    Ok(())
}

fn with_config(
    config: OAuthConfiguration,
) -> impl Filter<Extract = (OAuthConfiguration,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || config.clone())
}
