//! futures 0.2.x compatibility.
use std::io;
use std::sync::Arc;

use futures::{Async as Async01, Future as Future01, Poll as Poll01, Stream as Stream01};
use futures::task::{self as task01, Task as Task01};

use futures_core::{Async as Async02, Future as Future02, Stream as Stream02};
use futures_core::task::{Context, LocalMap, Wake, Waker};
use futures_core::executor::{Executor as Executor02};
use futures_io::{AsyncRead as AsyncRead02, AsyncWrite as AsyncWrite02};
use tokio_io::{AsyncRead as AsyncReadTk, AsyncWrite as AsyncWriteTk};

/// Wrap a `Future` from v0.2 as a `Future` from v0.1.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct Future02As01<E, F> {
    exec: E,
    v02: F,
}

/// Wrap a `Stream` from v0.2 as a `Stream` from v0.1.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct Stream02As01<E, S> {
    exec: E,
    v02: S,
}

/// Wrap some `AsyncRead` or `AsyncWrite` from v0.2 as the same from tokio-io.
#[derive(Debug)]
pub struct AsyncIo02AsTokio<E, S> {
    exec: E,
    v02: S,
}

/// A trait to convert any `Future` from v0.2 into a [`Future02As01`](Future02As01).
///
/// Implemented for all types that implement v0.2's `Future` automatically.
pub trait FutureInto01: Future02 {
    /// Converts this future into a `Future02As01`.
    ///
    /// An executor is required to allow this wrapped future to still access
    /// `Context::spawn` while wrapped.
    fn into_01_compat<E>(self, exec: E) -> Future02As01<E, Self>
    where
        Self: Sized,
        E: Executor02;
}

/// A trait to convert any `Stream` from v0.2 into a [`Stream02As01`](Stream02As01).
///
/// Implemented for all types that implement v0.2's `Stream` automatically.
pub trait StreamInto01: Stream02 {
    /// Converts this stream into a `Stream02As01`.
    ///
    /// An executor is required to allow this wrapped future to still access
    /// `Context::spawn` while wrapped.
    fn into_01_compat<E>(self, exec: E) -> Stream02As01<E, Self>
    where
        Self: Sized,
        E: Executor02;
}

/// A trait to convert any `AsyncRead`/`AsyncWrite` from v0.2 into a [`AsyncIo02AsTokio`](AsyncIo02AsTokio).
///
/// Implemented for all types that implement v0.2's `AsyncRead`/`AsyncWrite` automatically.
pub trait AsyncIoIntoTokio {
    /// Converts this IO into an `AsyncIo02AsTokio`.
    ///
    /// An executor is required to allow this wrapped future to still access
    /// `Context::spawn` while wrapped.
    fn into_tokio_compat<E>(self, exec: E) -> AsyncIo02AsTokio<E, Self>
    where
        Self: AsyncRead02 + AsyncWrite02 + Sized,
        E: Executor02;
}

impl<F> FutureInto01 for F
where
    F: Future02,
{
    fn into_01_compat<E>(self, exec: E) -> Future02As01<E, Self>
    where
        Self: Sized,
        E: Executor02,
    {
        Future02As01 {
            exec,
            v02: self,
        }
    }
}

impl<E, F> Future01 for Future02As01<E, F>
where
    F: Future02,
    E: Executor02,
{
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Poll01<Self::Item, Self::Error> {
        let mut locals = LocalMap::new();
        let waker = current_as_waker();
        let mut cx = Context::new(&mut locals, &waker, &mut self.exec);

        match self.v02.poll(&mut cx) {
            Ok(Async02::Ready(val)) => Ok(Async01::Ready(val)),
            Ok(Async02::Pending) => Ok(Async01::NotReady),
            Err(err) => Err(err),
        }
    }
}

impl<S> StreamInto01 for S
where
    S: Stream02,
{
    fn into_01_compat<E>(self, exec: E) -> Stream02As01<E, Self>
    where
        Self: Sized,
        E: Executor02,
    {
        Stream02As01 {
            exec,
            v02: self,
        }
    }
}

impl<E, S> Stream01 for Stream02As01<E, S>
where
    S: Stream02,
    E: Executor02,
{
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll01<Option<Self::Item>, Self::Error> {
        let mut locals = LocalMap::new();
        let waker = current_as_waker();
        let mut cx = Context::new(&mut locals, &waker, &mut self.exec);

        match self.v02.poll_next(&mut cx) {
            Ok(Async02::Ready(val)) => Ok(Async01::Ready(val)),
            Ok(Async02::Pending) => Ok(Async01::NotReady),
            Err(err) => Err(err),
        }
    }
}

impl<I> AsyncIoIntoTokio for I {
    fn into_tokio_compat<E>(self, exec: E) -> AsyncIo02AsTokio<E, Self>
    where
        Self: AsyncRead02 + AsyncWrite02 + Sized,
        E: Executor02,
    {
        AsyncIo02AsTokio {
            exec,
            v02: self,
        }
    }
}

impl<E: Executor02, I: AsyncRead02> io::Read for AsyncIo02AsTokio<E, I> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut locals = LocalMap::new();
        let waker = current_as_waker();
        let mut cx = Context::new(&mut locals, &waker, &mut self.exec);

        match self.v02.poll_read(&mut cx, buf) {
            Ok(Async02::Ready(val)) => Ok(val),
            Ok(Async02::Pending) => Err(would_block()),
            Err(err) => Err(err),
        }
    }
}

impl<E: Executor02, I: AsyncWrite02> io::Write for AsyncIo02AsTokio<E, I> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut locals = LocalMap::new();
        let waker = current_as_waker();
        let mut cx = Context::new(&mut locals, &waker, &mut self.exec);

        match self.v02.poll_write(&mut cx, buf) {
            Ok(Async02::Ready(val)) => Ok(val),
            Ok(Async02::Pending) => Err(would_block()),
            Err(err) => Err(err),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut locals = LocalMap::new();
        let waker = current_as_waker();
        let mut cx = Context::new(&mut locals, &waker, &mut self.exec);

        match self.v02.poll_flush(&mut cx) {
            Ok(Async02::Ready(val)) => Ok(val),
            Ok(Async02::Pending) => Err(would_block()),
            Err(err) => Err(err),
        }
    }
}

fn would_block() -> io::Error {
    io::Error::from(io::ErrorKind::WouldBlock)
}

impl<E: Executor02, I: AsyncRead02> AsyncReadTk for AsyncIo02AsTokio<E, I> {
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
        let init = self.v02.initializer();
        if init.should_initialize() {
            init.initialize(buf);
            true
        } else {
            false
        }
    }
}

impl<E: Executor02, I: AsyncWrite02> AsyncWriteTk for AsyncIo02AsTokio<E, I> {
    fn shutdown(&mut self) -> Poll01<(), io::Error> {
        let mut locals = LocalMap::new();
        let waker = current_as_waker();
        let mut cx = Context::new(&mut locals, &waker, &mut self.exec);

        match self.v02.poll_close(&mut cx) {
            Ok(Async02::Ready(val)) => Ok(Async01::Ready(val)),
            Ok(Async02::Pending) => Ok(Async01::NotReady),
            Err(err) => Err(err),
        }
    }
}

// Maybe it's possible to do all this without cloning and allocating,
// but I just wanted to get this working now. Optimzations welcome.

fn current_as_waker() -> Waker {
    Waker::from(Arc::new(Current(task01::current())))
}

struct Current(Task01);

impl Wake for Current {
    fn wake(arc_self: &Arc<Self>) {
        arc_self.0.notify();
    }
}
