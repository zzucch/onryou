mod connect;
mod default;
mod post;

use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use hyper::{Method, Request, Response};

pub async fn handle_request(
    request: Request<hyper::body::Incoming>,
    anki_media_directory: &str,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    const HOST: &str = "127.0.0.1";
    const PORT: u16 = 8765;

    log::trace!("request: {:?}", request);

    match *request.method() {
        Method::POST => post::handle(HOST, PORT, anki_media_directory, request).await,
        // i don't think this would be of any use but whatever
        Method::CONNECT => Ok(connect::handle(request)),
        _ => default::handle(HOST, PORT, request).await,
    }
}
