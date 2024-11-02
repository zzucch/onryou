use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::upgrade::Upgraded;

use hyper::Response;
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

// Received an HTTP request like:
// ```
// CONNECT www.domain.com:443 HTTP/1.1
// Host: www.domain.com:443
// Proxy-Connection: Keep-Alive
// ```
//
// When HTTP method is CONNECT we should return an empty body
// then we can eventually upgrade the connection and talk a new protocol.
//
// Note: only after client received an empty body with STATUS_OK can the
// connection be upgraded, so we can't return a response inside
// `on_upgrade` future.
pub fn handle(
    request: hyper::Request<hyper::body::Incoming>,
) -> Response<BoxBody<Bytes, hyper::Error>> {
    if let Some(address) = host_address(request.uri()) {
        tokio::task::spawn(async move {
            match hyper::upgrade::on(request).await {
                Ok(upgraded) => {
                    if let Err(err) = tunnel(upgraded, address).await {
                        log::error!("server io error: {}", err);
                    };
                }
                Err(err) => log::error!("upgrade error: {}", err),
            }
        });

        Response::new(empty())
    } else {
        log::error!("CONNECT host is not socket address: {:?}", request.uri());

        let mut response = Response::new(full("CONNECT must be to a socket address"));
        *response.status_mut() = hyper::StatusCode::BAD_REQUEST;

        response
    }
}

fn host_address(uri: &hyper::Uri) -> Option<String> {
    uri.authority().map(std::string::ToString::to_string)
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

async fn tunnel(upgraded: Upgraded, address: String) -> std::io::Result<()> {
    let mut server = TcpStream::connect(address).await?;
    let mut upgraded = TokioIo::new(upgraded);

    let (_, _) = tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;

    Ok(())
}
