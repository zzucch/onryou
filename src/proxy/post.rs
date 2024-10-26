use std::path::Path;

use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::body::{Buf, Bytes};
use hyper::client::conn::http1::Builder;
use hyper::{Request, Response};

use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

use crate::normalization;

pub async fn handle(
    host: &str,
    port: u16,
    anki_file_directory: &str,
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

    let modified_body_json = modify_body(anki_file_directory, &mut json).await.unwrap();
    let body_string = modified_body_json.to_string();

    log::trace!("parts: {:?}", parts);

    let mut request_builder = Request::builder()
        .method(parts.method)
        .uri(parts.uri)
        .version(parts.version);

    for (name, value) in &parts.headers {
        request_builder = request_builder.header(name, value);
    }

    let request = request_builder
        .body(Full::new(Bytes::from(body_string)))
        .unwrap();

    let response = sender.send_request(request).await?;

    Ok(response.map(http_body_util::BodyExt::boxed))
}

async fn modify_body<'a>(
    anki_file_directory: &str,
    body: &'a mut serde_json::Value,
) -> Result<&'a mut serde_json::Value, ()> {
    log::trace!("json body: {:?}", body);

    let action = body.get("action").unwrap().as_str().unwrap();

    match action {
        "addNote" | "updateNoteFields" => {
            if let Some(fields) = body
                .get_mut("params")
                .and_then(|params| params.get_mut("note"))
                .and_then(|note| note.get_mut("fields"))
                .and_then(|fields| fields.as_object_mut())
            {
                for (_, value) in fields.iter_mut() {
                    if let Some(field_value) = value.as_str() {
                        if field_value.starts_with("[sound:") {
                            let filename = field_value
                                .trim_start_matches("[sound:")
                                .trim_end_matches(']');
                            let file_path = format!("{anki_file_directory}/{filename}");

                            log::debug!("found sound file path: {}", file_path);

                            normalization::normalize_audio_file(Path::new(&file_path)).await;
                        }
                    }
                }
            };
        }
        _ => {}
    }

    Ok(body)
}
