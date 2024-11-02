use std::path::Path;

use anyhow::{Context, Result};
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::body::{Buf, Bytes};
use hyper::client::conn::http1::Builder;
use hyper::{Request, Response};

use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

use crate::normalization;

const SOUND_FIELD_START: &str = "[sound:";
const SOUND_FIELD_END: char = ']';

pub async fn handle(
    host: &str,
    port: u16,
    anki_media_directory: &str,
    request: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    log::debug!("handling post");

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

    let (parts, body) = request.into_parts();
    let collected_body = body.collect().await?.aggregate();

    let body_json: Result<serde_json::Value, _> = serde_json::from_reader(collected_body.reader());
    let mut json = body_json?;

    let modified_body_json = modify_body(anki_media_directory, &mut json).await?;
    let body_string = modified_body_json.to_string();

    log::trace!("parts: {:?}", parts);

    let mut request_builder = Request::builder()
        .method(parts.method)
        .uri(parts.uri)
        .version(parts.version);

    for (name, value) in &parts.headers {
        request_builder = request_builder.header(name, value);
    }

    let request = request_builder.body(Full::new(Bytes::from(body_string)))?;

    let response = sender.send_request(request).await?;

    Ok(response.map(http_body_util::BodyExt::boxed))
}

async fn modify_body<'a>(
    anki_media_directory: &str,
    body: &'a mut serde_json::Value,
) -> Result<&'a mut serde_json::Value> {
    log::trace!("json body: {:?}", body);

    let action = body
        .get("action")
        .context("every ankiconnect request must contain an action")?
        .as_str()
        .context("request field 'action' must contain some value")?;

    match action {
        "addNote" | "updateNoteFields" => process_note_fields(anki_media_directory, body).await?,
        _ => {}
    }

    Ok(body)
}

async fn process_note_fields<'a>(
    anki_media_directory: &str,
    body: &'a mut serde_json::Value,
) -> Result<(), anyhow::Error> {
    if let Some(fields) = body
        .get_mut("params")
        .and_then(|params| params.get_mut("note"))
        .and_then(|note| note.get_mut("fields"))
        .and_then(|fields| fields.as_object_mut())
    {
        for (_, value) in fields.iter_mut() {
            if let Some(field_value) = value.as_str() {
                if field_value.starts_with(SOUND_FIELD_START) {
                    process_sound_field(field_value, anki_media_directory).await?;
                }
            }
        }
    };

    Ok(())
}

async fn process_sound_field(
    field_value: &str,
    anki_media_directory: &str,
) -> Result<(), anyhow::Error> {
    let filename = field_value
        .trim_start_matches(SOUND_FIELD_START)
        .trim_end_matches(SOUND_FIELD_END);
    let file_path = format!("{anki_media_directory}/{filename}");

    log::debug!("sound file path: {}", file_path);

    normalization::normalize_audio_file(Path::new(&file_path)).await?;

    Ok(())
}
