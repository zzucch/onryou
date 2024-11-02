use std::env;

#[tokio::main]
async fn main() {
    const ANKICONNECT_URL: &str = "http://127.0.0.1:8765";

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let anki_media_directory_path =
        match onryou::anki_path::get_media_directory(ANKICONNECT_URL).await {
            Ok(path) => path.to_string(),
            Err(err) => {
                log::error!("failed to retrieve anki media directory: {:?}", err);
                return;
            }
        };

    let address = std::net::SocketAddr::from(([127, 0, 0, 1], 8100));
    let listener = match tokio::net::TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(err) => {
            log::error!(
                "failed to bind to address {}: {:?}. ensure the port is \
                available and not in use by another application",
                address,
                err
            );
            return;
        }
    };

    log::info!("listening on http://{}", address);

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(connection) => connection,
            Err(err) => {
                log::error!(
                    "failed to accept incoming connection: {:?}. \
                    this might be a network or resource issue.",
                    err
                );
                continue;
            }
        };
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
