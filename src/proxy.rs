mod connect;
mod post;

use connect::handle_connect;
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::body::Bytes;
use hyper::client::conn::http1::Builder;
use hyper::{Method, Request, Response};

use hyper_util::rt::TokioIo;
use post::handle_post;
use tokio::net::TcpStream;

pub async fn handle_request(
    request: Request<hyper::body::Incoming>,
    anki_media_directory: &str,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    log::trace!("request: {:?}", request);

    const HOST: &str = "127.0.0.1";
    const PORT: u16 = 8765;

    match *request.method() {
        Method::POST => handle_post(HOST, PORT, anki_media_directory, request).await,
        // i don't think this would be of any use but whatever
        Method::CONNECT => handle_connect(request),
        _ => {
            let stream = TcpStream::connect((HOST, PORT)).await.unwrap();
            let io = TokioIo::new(stream);

            let (mut sender, conn) = Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .handshake(io)
                .await?;

            tokio::task::spawn(async move {
                if let Err(err) = conn.await {
                    log::error!("connection failed: {:?}", err);
                }
            });

            let response = sender.send_request(request).await?;

            Ok(response.map(|b| b.boxed()))
        }
    }
}
