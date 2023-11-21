use std::{convert::Infallible, net::SocketAddr};

use bytes::Bytes;
use futures::TryStreamExt;
use http_body_util::{combinators::BoxBody, BodyExt, StreamBody};
use hyper::{body::Frame, service::service_fn, Response};
use hyper_util::rt::TokioIo;
use tokio::{net::TcpListener, sync::broadcast, time};
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};

async fn hello(
    rx: broadcast::Receiver<Bytes>,
) -> Result<Response<BoxBody<Bytes, BroadcastStreamRecvError>>, Infallible> {
    let stream = StreamBody::new(BroadcastStream::new(rx).map_ok(Frame::data));
    Ok(Response::new(stream.boxed()))
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
