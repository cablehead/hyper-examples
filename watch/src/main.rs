use std::{convert::Infallible, net::SocketAddr};

use bytes::Bytes;
use futures::StreamExt;
use http_body_util::{combinators::BoxBody, StreamBody};
use hyper::{body::Frame, service::service_fn, Response};
use hyper_util::rt::TokioIo;
use tokio::{net::TcpListener, sync::watch, time};
use tokio_stream::wrappers::WatchStream;

async fn hello(
    rx: watch::Receiver<Bytes>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
    let stream = WatchStream::new(rx)
        .map(Frame::data)
        .map(Ok::<_, Infallible>);
    let body = BoxBody::new(StreamBody::new(stream));
    let resp = Response::builder()
        .header("Content-Type", "text/plain")
        .body(body)
        .unwrap();
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    let (tx, rx) = watch::channel(Bytes::new());

    tokio::spawn(async move {
        let mut interval = time::interval(time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let now = chrono::Local::now().to_string();
            eprintln!("{:?}", now);
            let now_bytes = Bytes::from(now);
            let _ = tx.send(now_bytes);
        }
    });

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        tokio::task::spawn({
            let rx = rx.clone();
            async move {
                if let Err(err) = hyper::server::conn::http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |_req| {
                            let rx = rx.clone();
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
