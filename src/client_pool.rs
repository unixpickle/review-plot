use std::{
    collections::VecDeque,
    error::Error,
    fmt::Display,
    future::Future,
    mem::{swap, take},
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use super::client::Client;
use thirtyfour::error::WebDriverResult;
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug)]
pub enum PoolError {
    PoolClosed,
}

impl Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::PoolClosed => write!(f, "client pool is closed"),
        }
    }
}

impl Error for PoolError {}

pub async fn new_client_pool(capacity: usize, driver: &str) -> WebDriverResult<ObjectPool<Client>> {
    let mut objs = Vec::new();
    for _ in 0..capacity {
        let obj = Client::new(driver).await?;
        objs.push(obj);
    }
    Ok(ObjectPool {
        inner: Arc::new(Mutex::new(ObjectPoolInner {
            closed: false,
            capacity: capacity,
            waiting: VecDeque::new(),
            free: objs,
        })),
    })
}

pub struct ObjectPool<T> {
    inner: Arc<Mutex<ObjectPoolInner<T>>>,
}

impl<T> Clone for ObjectPool<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> ObjectPool<T> {
    pub async fn get(&self) -> Result<PoolHandle<T>, PoolError> {
        let (tx, rx) = channel(1);
        let tx_arc = Arc::new(tx);
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.closed {
                return Err(PoolError::PoolClosed);
            }
            if let Some(obj) = inner.free.pop() {
                drop(inner);
                return Ok(PoolHandle {
                    pool_inner: self.inner.clone(),
                    obj: Some(obj),
                });
            }
            inner.waiting.push_back(tx_arc.clone());
        }
        let mut waiter = PoolWaiter::<T>::new(self.inner.clone(), tx_arc, rx);
        waiter.recv().await
    }

    pub async fn close<F, Fut, E: Error>(&self, f: F) -> Result<(), E>
    where
        Fut: Future<Output = Result<(), E>>,
        F: Fn(T) -> Fut,
    {
        let mut free = Vec::new();
        let mut rx;
        let tx;
        let remaining;
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.closed {
                return Ok(());
            }
            inner.closed = true;
            swap(&mut free, &mut inner.free);
            remaining = inner.capacity - free.len();
            inner.waiting.clear();
            (tx, rx) = channel(remaining.max(1));
            for _ in 0..remaining {
                inner.waiting.push_back(Arc::new(tx.clone()));
            }
        }
        for client in free {
            f(client).await?;
        }
        for _ in 0..remaining {
            f(rx.recv().await.unwrap()).await?;
        }
        Ok(())
    }
}

pub struct PoolHandle<T> {
    pool_inner: Arc<Mutex<ObjectPoolInner<T>>>,
    obj: Option<T>,
}

impl<T> Drop for PoolHandle<T> {
    fn drop(&mut self) {
        let obj = take(&mut self.obj).unwrap();
        let mut inner = self.pool_inner.lock().unwrap();
        inner.return_object(obj);
    }
}

impl<T> Deref for PoolHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.obj.as_ref().unwrap()
    }
}

impl<T> DerefMut for PoolHandle<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.obj.as_mut().unwrap()
    }
}

struct PoolWaiter<T> {
    pool: Arc<Mutex<ObjectPoolInner<T>>>,
    tx: Arc<Sender<T>>,
    rx: Option<Receiver<T>>,
}

impl<T> PoolWaiter<T> {
    pub fn new(pool: Arc<Mutex<ObjectPoolInner<T>>>, tx: Arc<Sender<T>>, rx: Receiver<T>) -> Self {
        PoolWaiter {
            pool: pool,
            tx: tx,
            rx: Some(rx),
        }
    }

    pub async fn recv(&mut self) -> Result<PoolHandle<T>, PoolError> {
        let obj = self
            .rx
            .as_mut()
            .unwrap()
            .recv()
            .await
            .ok_or(PoolError::PoolClosed)?;
        self.rx = None;
        Ok(PoolHandle {
            pool_inner: self.pool.clone(),
            obj: Some(obj),
        })
    }
}

impl<T> Drop for PoolWaiter<T> {
    fn drop(&mut self) {
        if let Some(mut rx) = take(&mut self.rx) {
            let mut inner = self.pool.lock().unwrap();

            // Remove ourselves from the queue.
            let mut i = 0;
            while i < inner.waiting.len() {
                if Arc::ptr_eq(&inner.waiting[i], &self.tx) {
                    inner.waiting.remove(i);
                    break;
                }
                i += 1;
            }

            // We might have been sent a object but never received
            // it, in which case we should free it.
            if let Ok(obj) = rx.try_recv() {
                inner.return_object(obj);
            }
        }
    }
}

struct ObjectPoolInner<T> {
    closed: bool,
    capacity: usize,
    waiting: VecDeque<Arc<Sender<T>>>,
    free: Vec<T>,
}

impl<T> ObjectPoolInner<T> {
    pub fn return_object(&mut self, obj: T) {
        if let Some(waiting) = self.waiting.pop_front() {
            // The buffer should never be full, and the
            // other side of the channel won't be dropped
            // unless it's removed from the waiting queue.
            waiting.try_send(obj).unwrap();
        } else {
            self.free.push(obj);
        }
    }
}
