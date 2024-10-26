mod connect;
mod post;

use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use hyper::client::conn::http1::Builder;
use hyper::{Method, Request, Response};

use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

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

            Ok(response.map(http_body_util::BodyExt::boxed))
        }
    }
}
