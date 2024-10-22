use std::net::SocketAddr;

use hyper::server::conn::http1;
use hyper::service::service_fn;

use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

// TODO: make proper auto-search or configuration
const ANKI_MEDIA_DIRECTORY: &str = "/home/zcchr/.local/share/Anki2/User 1/collection.media";

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
                .serve_connection(
                    io,
                    service_fn(move |request| {
                        onryou::proxy::handle_request(request, ANKI_MEDIA_DIRECTORY)
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
