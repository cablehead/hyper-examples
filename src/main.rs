use std::convert::Infallible;
use std::net::SocketAddr;

use http_body_util::StreamBody;
use hyper::body::Bytes;
use hyper::{service::service_fn, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::time;
use tokio_stream::wrappers::BroadcastStream;

async fn hello(
    rx: broadcast::Receiver<Bytes>,
) -> Result<Response<StreamBody<BroadcastStream<Bytes>>>, Infallible> {
    let stream = BroadcastStream::new(rx);
    Ok(Response::new(StreamBody::new(stream)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    let (tx, _) = broadcast::channel(10);

    tokio::spawn(async move {
        let mut interval = time::interval(time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let now = chrono::Local::now().to_string();
            let now_bytes = Bytes::from(now);
            if tx.send(now_bytes).is_err() {
                break;
            }
        }
    });

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            let rx = tx.subscribe();
            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service_fn(move |_req| hello(rx)))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
