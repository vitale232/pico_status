#[macro_use]
extern crate dotenv_codegen;

use std::collections::HashMap;

use graph_oauth::oauth::AccessToken;
use graph_rs_sdk::oauth::OAuth;
use warp::Filter;

static CLIENT_ID: &str = dotenv!("CLIENT_ID");
static CLIENT_SECRET: &str = dotenv!("CLIENT_SECRET");
static TENANT_ID: &str = dotenv!("TENANT_ID");
static CONFIG: &Config = &Config::new(CLIENT_ID, CLIENT_SECRET, TENANT_ID);

#[derive(Clone, Debug)]
pub struct Config<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub tenant_id: &'a str,
    pub port: u16,
}

impl<'a> Config<'a> {
    const fn new(client_id: &'a str, client_secret: &'a str, tenant_id: &'a str) -> Self {
        Config {
            client_id,
            client_secret,
            tenant_id,
            port: 42069,
        }
    }
}

#[tokio::main]
async fn main() {
    // Get the oauth client and request a browser sign in
    let mut oauth = oauth_web_client(CONFIG);
    let mut request = oauth.build().token_flow();
    request.browser_authorization().open().unwrap();

    let token_query = warp::get().and(warp::path("redirect")).map(|| {
        warp::reply::html(
            r"
<html>
  <head>
    <title>HTML with warp!</title>
  </head>
  <body>
    <h1>warp + HTML = :heart:</h1>
    <div id='status'></div>
  </body>
  <script>
    function handleHash() {
      let url = window.location.href;
      let parts = url.split('#');
      let body = '';
      if (parts.length < 2) {
        document.querySelector('#status').innerHTML =
          'Something went wrong. No HASH in URL.';
        return;
      } else {
        body = parts[1];
        fetch('http://localhost:42069/token', {
          method: 'POST',
          headers: {
            'content-type': 'application/x-www-form-urlencoded',
          },
          body,
        })
          .then((res) => res.text())
          .then((res) => console.log({ res }))
          .catch((err) => console.error({ err }));
      }
    }
    handleHash();
  </script>
</html>
",
        )
    });

    let token_response = warp::post()
        .and(warp::path("token"))
        .and(warp::body::form())
        .and(with_oauth(oauth))
        .map(|simple_map: HashMap<String, String>, mut oauth: OAuth| {
            println!("map: {:?}", simple_map);
            let access_token = simple_map
                .get("access_token")
                .expect("Access token not in Map!");
            let expires_in = simple_map
                .get("expires_in")
                .expect("`expires_in` not in map")
                .parse::<i64>()
                .expect("Could not parse `expires_in` to i64!");
            let scope = simple_map.get("scope").expect("`scope` not in Map!");
            let token_type = simple_map
                .get("token_type")
                .expect("`token_type` not in Map!");
            oauth.access_token(AccessToken::new(
                token_type,
                expires_in,
                scope,
                access_token,
            ));
            println!("Hydrated: {:?}", oauth);
            "Got a urlencoded body!"
        });

    let routes = token_query.or(token_response);
    warp::serve(routes).run(([127, 0, 0, 1], CONFIG.port)).await;
    println!("sleeping...");
    tokio::time::sleep(tokio::time::Duration::from_secs(69)).await;
}

fn with_oauth(
    oauth: OAuth,
) -> impl Filter<Extract = (OAuth,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || oauth.clone())
}

fn oauth_web_client(config: &Config) -> OAuth {
    let mut oauth = OAuth::new();
    oauth
        .client_id(config.client_id)
        .client_secret(config.client_secret)
        .add_scope("Presence.Read")
        .add_scope("Calendars.Read")
        .redirect_uri(&format!("http://localhost:{}/redirect", config.port))
        .authorize_url(&format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize",
            &config.tenant_id
        ))
        .access_token_url(&format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            &config.tenant_id
        ))
        .refresh_token_url(&format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            &config.tenant_id
        ))
        .response_type("token")
        .post_logout_redirect_uri(&format!("http://localhost:{}/redirect", config.port));
    // .grant_type("authorizaton_code");
    oauth
}
