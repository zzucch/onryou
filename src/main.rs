use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const ANKICONNECT_URL: &str = "http://127.0.0.1:8765";

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let anki_media_directory_path = onryou::anki_path::get_media_directory(ANKICONNECT_URL)
        .await
        .unwrap()
        .to_string()
        .clone();
    log::info!("using media directory {}", anki_media_directory_path);

    let address = std::net::SocketAddr::from(([127, 0, 0, 1], 8100));
    let listener = tokio::net::TcpListener::bind(address).await?;
    log::info!("listening on http://{}", address);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = hyper_util::rt::TokioIo::new(stream);
        let anki_media_directory_path = anki_media_directory_path.clone();

        tokio::task::spawn(async move {
            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .title_case_headers(true)
                .preserve_header_case(true)
                .serve_connection(
                    io,
                    hyper::service::service_fn(|request| {
                        onryou::proxy::handle_request(request, &anki_media_directory_path)
                    }),
                )
                .with_upgrades()
                .await
            {
                log::error!("failed to serve connection: {:?}", err);
            }
        });
    }
}
