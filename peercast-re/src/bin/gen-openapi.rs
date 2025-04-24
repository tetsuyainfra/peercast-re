use libpeercast_re::http::Api;
use peercast_re::api::{router, ApiDoc, ReStore};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;



fn main() {
    let (_,  api ) = router(ReStore{}.into());

    let j = api.to_pretty_json().unwrap();
    println!("{j}");

}
