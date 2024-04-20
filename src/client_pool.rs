use std::{
    collections::VecDeque,
    mem::take,
    ops::Deref,
    sync::{Arc, Mutex},
};

use super::client::Client;
use thirtyfour::error::WebDriverResult;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub async fn new_client_pool(capacity: usize, driver: &str) -> WebDriverResult<ObjectPool<Client>> {
    let mut objs = Vec::new();
    for _ in 0..capacity {
        let obj = Client::new(driver).await?;
        objs.push(obj);
    }
    Ok(ObjectPool {
        inner: Arc::new(Mutex::new(ObjectPoolInner {
            waiting: VecDeque::new(),
            free: objs,
        })),
    })
}

pub struct ObjectPool<T> {
    inner: Arc<Mutex<ObjectPoolInner<T>>>,
}

impl<T> ObjectPool<T> {
    pub async fn get(&self) -> PoolHandle<T> {
        let (tx, rx) = channel(1);
        let tx_arc = Arc::new(tx);
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(obj) = inner.free.pop() {
                drop(inner);
                return PoolHandle {
                    pool_inner: self.inner.clone(),
                    obj: Some(obj),
                };
            }
            inner.waiting.push_back(tx_arc.clone());
        }
        let mut waiter = PoolWaiter::new(self.inner.clone(), tx_arc, rx);
        waiter.recv().await
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

    pub async fn recv(&mut self) -> PoolHandle<T> {
        let obj = self.rx.as_mut().unwrap().recv().await.unwrap();
        self.rx = None;
        PoolHandle {
            pool_inner: self.pool.clone(),
            obj: Some(obj),
        }
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
