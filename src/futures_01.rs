use std::{mem, ptr};

use futures_compat_01::{Async as Async01, Future as Future01};
use futures_compat_01::executor::{Notify, NotifyHandle, UnsafeNotify, with_notify};

use futures_compat_02::{Async as Async02, Future as Future02, Poll as Poll02};
use futures_compat_02::task::{Context, Waker};

pub struct Future01As02<F> {
    v01: F,
}

pub trait FutureInto02: Future01 {
    fn into_02_compat(self) -> Future01As02<Self> where Self: Sized;
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
        with_context(cx, || {
            match self.v01.poll() {
                Ok(Async01::Ready(val)) => Ok(Async02::Ready(val)),
                Ok(Async01::NotReady) => Ok(Async02::Pending),
                Err(err) => Err(err),
            }
        })
    }
}

pub fn with_context<F, R>(cx: &mut Context, f: F) -> R
where
    F: FnOnce() -> R,
{
    with_notify(&WakerToHandle(cx.waker()), 0, f)
}

struct NotifyWaker(Waker);

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
