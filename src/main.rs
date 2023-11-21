use std::convert::Infallible;
use std::net::SocketAddr;

use hyper::body::Frame;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use bytes::Bytes;
use http_body::Body;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::time::{self, Interval};

#[derive(Debug)]
struct HelloWorldBody {
    interval: Interval,
}

impl Body for HelloWorldBody {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match Pin::new(&mut self.interval).poll_tick(cx) {
            Poll::Ready(_) => {
                let frame = Frame::data(Bytes::from("Hello world"));
                eprintln!("pong");
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for HelloWorldBody {
    fn drop(&mut self) {
        eprintln!("HelloWorldBody dropped");
    }
}

async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<HelloWorldBody>, Infallible> {
    let body = HelloWorldBody {
        interval: time::interval(time::Duration::from_secs(1)),
    };
    Ok(Response::new(body))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        tokio::task::spawn(async move {
            if let Err(err) =
                hyper::server::conn::http1::Builder::new()
                    .serve_connection(io, service_fn(hello))
                    .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
