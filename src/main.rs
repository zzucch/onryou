use std::net::SocketAddr;

use hyper::server::conn::http1;
use hyper::service::service_fn;

use hyper_util::rt::TokioIo;
use onryou::anki_path::get_anki_media_directory;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // TODO: perhaps request permission beforehand?
    let anki_media_directory = get_anki_media_directory();
    let anki_media_directory_path_str = anki_media_directory.await.unwrap().to_string().clone();
    log::info!("using media directory {}", anki_media_directory_path_str);

    let address = SocketAddr::from(([127, 0, 0, 1], 8100));
    let listener = TcpListener::bind(address).await?;
    log::info!("listening on http://{}", address);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let anki_media_directory_path_str = anki_media_directory_path_str.clone();

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .title_case_headers(true)
                .preserve_header_case(true)
                .serve_connection(
                    io,
                    service_fn(|req| {
                        onryou::proxy::handle_request(req, &anki_media_directory_path_str)
                    }),
                )
                .with_upgrades()
                .await
            {
                log::error!("Failed to serve connection: {:?}", err);
            }
        });
    }
}
