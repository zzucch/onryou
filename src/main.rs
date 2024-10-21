use std::net::SocketAddr;

use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::client::conn::http1::Builder;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Method, Request, Response, StatusCode, Uri};

use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let address = SocketAddr::from(([127, 0, 0, 1], 8100));

    let listener = TcpListener::bind(address).await?;
    log::info!("listening on http://{}", address);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, service_fn(proxy))
                .with_upgrades()
                .await
            {
                log::error!("failed to serve connection: {:?}", err);
            }
        });
    }
}

async fn proxy(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    log::debug!("req: {:?}", req);

    const HOST: &str = "127.0.0.1";
    const PORT: u16 = 8765;

    match req.method() {
        &Method::POST => handle_post(HOST, PORT, req).await,
        // i don't think this would be of any use but whatever
        &Method::CONNECT => handle_connect(req),
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

            let resp = sender.send_request(req).await?;

            Ok(resp.map(|b| b.boxed()))
        }
    }
}

async fn handle_post(
    host: &str,
    port: u16,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    log::debug!("hadling post");

    let stream = TcpStream::connect((host, port)).await.unwrap();
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

    let (parts, body) = req.into_parts();
    let collected_body = body.collect().await?;
    let body_bytes = collected_body.to_bytes();
    let body_str =
        String::from_utf8(body_bytes.to_vec()).unwrap_or_else(|_| "<invalid UTF-8>".to_string());

    log::debug!("parts: {:?}\nbody: {:?}", parts, body_str);

    let mut request_builder = Request::builder()
        .method(parts.method)
        .uri(parts.uri)
        .version(parts.version);

    for (name, value) in parts.headers.iter() {
        request_builder = request_builder.header(name, value);
    }

    let request = request_builder.body(Empty::<Bytes>::new()).unwrap();

    let resp = sender.send_request(request).await?;

    Ok(resp.map(|b| b.boxed()))
}

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
fn handle_connect(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match host_address(req.uri()) {
        Some(address) => {
            tokio::task::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        if let Err(err) = tunnel(upgraded, address).await {
                            log::error!("server io error: {}", err);
                        };
                    }
                    Err(err) => log::error!("upgrade error: {}", err),
                }
            });

            Ok(Response::new(empty()))
        }
        None => {
            log::error!("CONNECT host is not socket address: {:?}", req.uri());

            let mut resp = Response::new(full("CONNECT must be to a socket address"));
            *resp.status_mut() = StatusCode::BAD_REQUEST;

            Ok(resp)
        }
    }
}

fn host_address(uri: &Uri) -> Option<String> {
    uri.authority().and_then(|auth| Some(auth.to_string()))
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
