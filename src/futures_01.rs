//! futures 0.1.x compatibility.
use std::{mem, io, ptr};

use futures::{Async as Async01, Future as Future01, Poll as Poll01, Stream as Stream01};
use futures::executor::{Notify, NotifyHandle, UnsafeNotify, with_notify};

use futures_core::{Async as Async02, Future as Future02, Poll as Poll02, Stream as Stream02};
use futures_core::task::{Context, Waker};
use futures_io::{AsyncRead as AsyncRead02, AsyncWrite as AsyncWrite02};

use tokio_io::{AsyncRead as AsyncReadTk, AsyncWrite as AsyncWriteTk};

/// Wrap a `Future` from v0.1 as a `Future` from v0.2.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct Future01As02<F> {
    v01: F,
}

/// Wrap a `Stream` from v0.1 as a `Stream` from v0.2.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct Stream01As02<S> {
    v01: S,
}

/// Wrap a IO from tokio-io as an `AsyncRead`/`AsyncWrite` from v0.2.
#[derive(Debug)]
pub struct TokioAsAsyncIo02<I> {
    v01: I,
}

/// A trait to convert any `Future` from v0.1 into a [`Future01As02`](Future01As02).
///
/// Implemented for all types that implement v0.1's `Future` automatically.
pub trait FutureInto02: Future01 {
    /// Converts this future into a `Future01As02`.
    fn into_02_compat(self) -> Future01As02<Self> where Self: Sized;
}
/// A trait to convert any `Stream` from v0.1 into a [`Stream01As02`](Stream01As02).
///
/// Implemented for all types that implement v0.1's `Stream` automatically.
pub trait StreamInto02: Stream01 {
    /// Converts this stream into a `Stream01As02`.
    fn into_02_compat(self) -> Stream01As02<Self> where Self: Sized;
}

/// A trait to convert any `AsyncRead`/`AsyncWrite` from tokio-io into a [`TokioAsAsyncIo02`](TokioAsAsyncIo02).
///
/// Implemented for all types that implement tokio-io's `AsyncRead`/`AsyncWrite` automatically.
pub trait TokioIntoAsyncIo02 {
    /// Converts this IO into an `TokioAsAsyncIo02`.
    fn into_v02_compat(self) -> TokioAsAsyncIo02<Self>
    where
        Self: AsyncReadTk + AsyncWriteTk + Sized;
}

impl<F> FutureInto02 for F
where
    F: Future01,
{
    fn into_02_compat(self) -> Future01As02<Self>
    where
        Self: Sized,
    {
        Future01As02 {
            v01: self,
        }
    }
}

impl<F> Future02 for Future01As02<F>
where
    F: Future01,
{
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self, cx: &mut Context) -> Poll02<Self::Item, Self::Error> {
        with_context_poll(cx, || self.v01.poll())
    }
}

impl<S> StreamInto02 for S
where
    S: Stream01,
{
    fn into_02_compat(self) -> Stream01As02<Self>
    where
        Self: Sized,
    {
        Stream01As02 {
            v01: self,
        }
    }
}

impl<S> Stream02 for Stream01As02<S>
where
    S: Stream01,
{
    type Item = S::Item;
    type Error = S::Error;

    fn poll_next(&mut self, cx: &mut Context) -> Poll02<Option<Self::Item>, Self::Error> {
        with_context_poll(cx, || self.v01.poll())
    }
}

impl<I> TokioIntoAsyncIo02 for I {
    fn into_v02_compat(self) -> TokioAsAsyncIo02<Self>
    where
        Self: AsyncReadTk + AsyncWriteTk + Sized,
    {
        TokioAsAsyncIo02 {
            v01: self,
        }
    }
}

impl<I: AsyncReadTk> AsyncRead02 for TokioAsAsyncIo02<I> {
    fn poll_read(&mut self, cx: &mut Context, buf: &mut [u8]) -> Poll02<usize, io::Error> {
        with_context_poll(cx, || self.v01.poll_read(buf))
    }
}

impl<I: AsyncWriteTk> AsyncWrite02 for TokioAsAsyncIo02<I> {
    fn poll_write(&mut self, cx: &mut Context, buf: &[u8]) -> Poll02<usize, io::Error> {
        with_context_poll(cx, || self.v01.poll_write(buf))
    }

    fn poll_flush(&mut self, cx: &mut Context) -> Poll02<(), io::Error> {
        with_context_poll(cx, || self.v01.poll_flush())
    }

    fn poll_close(&mut self, cx: &mut Context) -> Poll02<(), io::Error> {
        with_context_poll(cx, || self.v01.shutdown())
    }
}

/// Execute a function with the context used as a v0.1 `Notifier`.
pub fn with_context<F, R>(cx: &mut Context, f: F) -> R
where
    F: FnOnce() -> R,
{
    with_notify(&WakerToHandle(cx.waker()), 0, f)
}

/// Execute a function with the context used as a v0.1 `Notifier`, converting
/// v0.1 `Poll` into v0.2 version.
pub fn with_context_poll<F, R, E>(cx: &mut Context, f: F) -> Poll02<R, E>
where
    F: FnOnce() -> Poll01<R, E>,
{
    with_context(cx, move || {
        match f() {
            Ok(Async01::Ready(val)) => Ok(Async02::Ready(val)),
            Ok(Async01::NotReady) => Ok(Async02::Pending),
            Err(err) => Err(err),
        }
    })
}

struct NotifyWaker(Waker);

#[allow(missing_debug_implementations)]
#[derive(Clone)]
struct WakerToHandle<'a>(&'a Waker);

#[doc(hidden)]
impl<'a> From<WakerToHandle<'a>> for NotifyHandle {
    fn from(handle: WakerToHandle<'a>) -> NotifyHandle {
        unsafe {
            let ptr = NotifyWaker(handle.0.clone());
            let ptr = mem::transmute::<NotifyWaker, *mut UnsafeNotify>(ptr);
            NotifyHandle::new(ptr)
        }
    }
}

impl Notify for NotifyWaker {
    fn notify(&self, _: usize) {
        unsafe {
            let me: *const NotifyWaker = self;
            (&*me).0.wake();
        }
    }
}

unsafe impl UnsafeNotify for NotifyWaker {
    unsafe fn clone_raw(&self) -> NotifyHandle {
        let me: *const NotifyWaker = self;
        WakerToHandle(&(&*me).0).into()
    }

    unsafe fn drop_raw(&self) {
        let mut me: *const NotifyWaker = self;
        let me = &mut me as *mut *const NotifyWaker;
        ptr::drop_in_place(me);
    }
}
