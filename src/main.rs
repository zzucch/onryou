use std::net::SocketAddr;

use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::{Buf, Bytes};
use hyper::client::conn::http1::Builder;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Method, Request, Response, StatusCode, Uri};

use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};

// TODO: make proper auto-search or configuration
const ANKI_FILE_DIR: &str = "~/.local/share/Anki2/User 1/collection.media";

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
    request: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    log::debug!("request: {:?}", request);

    const HOST: &str = "127.0.0.1";
    const PORT: u16 = 8765;

    match request.method() {
        &Method::POST => handle_post(HOST, PORT, request).await,
        // i don't think this would be of any use but whatever
        &Method::CONNECT => handle_connect(request),
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

async fn handle_post(
    host: &str,
    port: u16,
    request: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    log::debug!("handling post");

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

    let (parts, body) = request.into_parts();
    let collected_body = body.collect().await?.aggregate();

    let body_json: Result<serde_json::Value, _> = serde_json::from_reader(collected_body.reader());

    let mut json = body_json.unwrap();

    let modified_body_json = modify_body(&mut json);

    let modified_body_json = modified_body_json.unwrap();
    let body_string = modified_body_json.to_string();

    log::debug!("parts: {:?}", parts);

    let mut request_builder = Request::builder()
        .method(parts.method)
        .uri(parts.uri)
        .version(parts.version);

    for (name, value) in parts.headers.iter() {
        request_builder = request_builder.header(name, value);
    }

    let request = request_builder
        .body(Full::new(Bytes::from(body_string)))
        .unwrap();

    let response = sender.send_request(request).await?;

    Ok(response.map(|b| b.boxed()))
}

fn modify_body(body: &mut serde_json::Value) -> Result<&mut serde_json::Value, ()> {
    log::debug!("json body: {:?}", body);

    let action = body.get("action").unwrap().as_str().unwrap();

    match action {
        "updateNoteFields" => {
            body.get_mut("params")
                .and_then(|params| params.get_mut("note"))
                .and_then(|note| note.get_mut("fields"))
                .and_then(|fields| fields.as_object_mut())
                .map(|fields_map| {
                    for (_, value) in fields_map.iter_mut() {
                        if let Some(field_value) = value.as_str() {
                            if field_value.starts_with("[sound:") {
                                let filename = field_value
                                    .trim_start_matches("[sound:")
                                    .trim_end_matches(']');
                                let file_path = format!("{}/{}", ANKI_FILE_DIR, filename);

                                log::info!("found sound file path: {}", file_path);
                            }
                        }
                    }
                });
        }
        // perhaps modify it here?? "storeMediaFile" => {}
        _ => {}
    }

    Ok(body)
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
    request: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match host_address(request.uri()) {
        Some(address) => {
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

            Ok(Response::new(empty()))
        }
        None => {
            log::error!("CONNECT host is not socket address: {:?}", request.uri());

            let mut response = Response::new(full("CONNECT must be to a socket address"));
            *response.status_mut() = StatusCode::BAD_REQUEST;

            Ok(response)
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
