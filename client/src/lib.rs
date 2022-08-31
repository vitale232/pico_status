#[macro_use]
extern crate serde;

pub mod oauth {
    use std::sync::{Arc, Mutex};

    use reqwest::Client;
    use tokio::sync::oneshot;
    use warp::Filter;

    pub type OAuthConfiguration = Arc<Mutex<Config>>;

    pub async fn flow(
        config: OAuthConfiguration,
        scope: &str,
        client: &Client,
    ) -> Result<AccessToken, Box<dyn std::error::Error>> {
        let (killtx, killrx) = oneshot::channel::<u8>();
        let access_code = warp::get()
            .and(warp::path("redirect"))
            .and(with_config(config.clone()))
            .and(warp::query::<AccessCode>())
            .map(|c: OAuthConfiguration, ac: AccessCode| {
                c.lock().unwrap().set_access_code(&ac.code);
                ac.code
            });

        webbrowser::open(&config.lock().unwrap().get_authorize_url(scope))
            .expect("Could not open browser");

        let port = config.lock().unwrap().get_port();
        let (_, server) = warp::serve(access_code).bind_with_graceful_shutdown(
            ([127, 0, 0, 1], port),
            async move {
                println!("waiting for signal");
                killrx.await.expect("Error handling shutdown receiver");
                println!("got signal");
            },
        );

        tokio::spawn(async {
            let secs = 5;
            println!("Dooms day prepping, {} seconds to go...", secs);
            tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
            killtx.send(1).expect("Could not send kill signal!");
        });

        match tokio::join!(tokio::task::spawn(server)).0 {
            Ok(()) => println!("serving..."),
            Err(e) => println!("An error occurred in Join! {:?}", e),
        };

        let token_url = config.lock().unwrap().get_token_url();
        let body = config.lock().unwrap().to_token_request_body();
        let token = client
            .post(token_url)
            .form(&body)
            .send()
            .await?
            .json::<AccessToken>()
            .await?;

        println!("T: {:#?}", token);
        Ok(token)
    }

    fn with_config(
        config: OAuthConfiguration,
    ) -> impl Filter<Extract = (OAuthConfiguration,), Error = std::convert::Infallible> + Clone
    {
        warp::any().map(move || config.clone())
    }

    #[derive(Clone, Debug)]
    pub struct Config {
        pub client_id: String,
        pub client_secret: String,
        pub tenant_id: String,
        pub port: u16,
        pub access_code: Option<String>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct TokenRequestBody {
        client_id: String,
        redirect_uri: String,
        code: String,
        grant_type: String,
    }

    impl Config {
        pub fn new(client_id: &str, client_secret: &str, tenant_id: &str) -> Self {
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

        fn to_token_request_body(&self) -> TokenRequestBody {
            let code = match &self.access_code {
                Some(ac) => ac,
                None => panic!("No Access Code available."),
            };
            TokenRequestBody {
                code: code.into(),
                client_id: self.client_id.clone(),
                redirect_uri: self.get_redirect_uri(),
                grant_type: String::from("authorization_code"),
            }
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

    #[derive(Debug, Deserialize, Serialize)]
    struct AccessCode {
        pub code: String,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AccessToken {
        token_type: String,
        scope: String,
        expires_in: i64,
        ext_expires_in: i64,
        pub access_token: String,
        refresh_token: String,
    }
}
