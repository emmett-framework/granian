use std::{
    marker::{PhantomData, Unpin},
    pin::Pin,
    task::{Context, Poll},
};

use hyper_util::rt::TokioIo;

pub(crate) struct IOTypeNotSend<T> {
    _marker: PhantomData<*const ()>,
    stream: TokioIo<T>,
}

impl<T> IOTypeNotSend<T> {
    pub fn new(stream: TokioIo<T>) -> Self {
        Self {
            _marker: PhantomData,
            stream,
        }
    }
}

impl<T> hyper::rt::Write for IOTypeNotSend<T>
where
    T: tokio::io::AsyncWrite + Unpin,
{
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

impl<T> hyper::rt::Read for IOTypeNotSend<T>
where
    T: tokio::io::AsyncRead + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}
