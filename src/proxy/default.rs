use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper::client::conn::http1::Builder;

use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

pub async fn handle(
    host: &str,
    port: u16,
    request: hyper::Request<hyper::body::Incoming>,
) -> anyhow::Result<hyper::Response<BoxBody<Bytes, hyper::Error>>> {
    let stream = TcpStream::connect((host, port)).await?;
    let io = TokioIo::new(stream);

    let (mut sender, connection) = Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .handshake(io)
        .await?;

    tokio::task::spawn(async move {
        if let Err(err) = connection.await {
            log::error!("connection failed: {:?}", err);
        }
    });

    let response = sender.send_request(request).await?;

    Ok(response.map(BodyExt::boxed))
}
