#[allow(non_camel_case_types)]
#[derive(RustEmbed)]
#[folder = "client/dist/"]
#[include = "*.{html,js,css,svg}"]
struct Asset;

pub struct StaticFile<T>(pub T);
impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();
        let path = match path.as_str() {
            "" => "index.html",
            _ => &path,
        };
        trace!(?path);

        match Asset::get(path) {
            Some(content) => {
                trace!("StaticFile(path: {path}) discovered");
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            }
            None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
        }
    }
}
