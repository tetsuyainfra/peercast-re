use axum::{
    http::StatusCode, response::{Html, IntoResponse, Redirect}, routing::{get, post}, Router
};
use axum_extra::{
    TypedHeader,
    extract::cookie::{Cookie, CookieJar},
    headers::authorization::{Authorization, Bearer},
};
use tracing_subscriber::fmt::format;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/sessions", post(create_session))
        .route("/me", get(me))
        .route("/list", get(listing));

    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("http://{}/", addr);
    println!("http://{}/send", addr);

    axum::serve(listener, app).await.unwrap();
}

async fn root_handler(
    jar: CookieJar,
) -> (CookieJar, Html<String>) {
    let jar = jar.add(Cookie::new("session_id", "123"));
    let html = format!("
<body>
/ <br />
<a href='/me'>/me</a> <br />
<a href='/list'>/list</a> <br />
<button id='store'>Store</button>
<script>
let btn = document.getElementById('store');
btn.addEventListener('click', function(e) {{
    console.log('navigator.cookieEnabled', navigator.cookieEnabled);
    document.cookie = 'key2=123';
    document.cookie = 'key2=ABC';
    console.log(document.cookie);
}})
</script>
</body>
");

    (jar, Html(html))
}

async fn create_session(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), StatusCode> {
    if let Some(session_id) = authorize_and_create_session(auth.token()).await {
        Ok((
            // the updated jar must be returned for the changes
            // to be included in the response
            jar.add(Cookie::new("session_id", session_id)),
            Redirect::to("/me"),
        ))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn me(jar: CookieJar) -> Result<String, StatusCode> {
    if let Some(session_id) = jar.get("session_id") {
        Ok(format!("session_id: {}", session_id))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn listing(jar: CookieJar) -> Result<String, StatusCode> {
    // let mut v = Vec::<String>::new();
    // jar.iter().for_each(|c| {
    //     v.push(c.name().into());
    // });

    Ok(format!("{:#?}", jar))
}

async fn authorize_and_create_session(token: &str) -> Option<String> {
    // authorize the user and create a session...
    todo!()
}
