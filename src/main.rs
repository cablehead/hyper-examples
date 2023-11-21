use futures::TryStreamExt;

use std::convert::Infallible;
use std::net::SocketAddr;

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use http_body_util::StreamBody;
use hyper::body::Frame;
use hyper::{service::service_fn, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::time;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;

async fn hello(
    rx: broadcast::Receiver<Bytes>,
) -> Result<Response<BoxBody<Bytes, BroadcastStreamRecvError>>, Infallible> {
    let stream = StreamBody::new(BroadcastStream::new(rx).map_ok(Frame::data));
    let boxed_body = stream.boxed();

    let response = Response::builder()
        .status(StatusCode::OK)
        .body(boxed_body)
        .unwrap();

    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    let (tx, _) = broadcast::channel(10);

    tokio::spawn({
        let tx = tx.clone();
        async move {
            let mut interval = time::interval(time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                let now = chrono::Local::now().to_string();
                eprintln!("{:?}", now);
                let now_bytes = Bytes::from(now);
                let _ = tx.send(now_bytes);
            }
        }
    });

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        tokio::task::spawn({
            let tx = tx.clone();
            async move {
                if let Err(err) = hyper::server::conn::http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |_req| {
                            let rx = tx.subscribe();
                            hello(rx)
                        }),
                    )
                    .await
                {
                    println!("Error serving connection: {:?}", err);
                }
            }
        });
    }
}
