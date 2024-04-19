use std::{
    collections::VecDeque,
    mem::take,
    ops::Deref,
    sync::{Arc, Mutex},
};

use super::client::Client;
use thirtyfour::error::WebDriverResult;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct ClientPool {
    inner: Arc<Mutex<ClientPoolInner>>,
}

impl ClientPool {
    pub async fn new(capacity: usize, driver: &str) -> WebDriverResult<Self> {
        let mut clients = Vec::new();
        for _ in 0..capacity {
            let client = Client::new(driver).await?;
            clients.push(client);
        }
        Ok(ClientPool {
            inner: Arc::new(Mutex::new(ClientPoolInner {
                waiting: VecDeque::new(),
                free: clients,
            })),
        })
    }

    pub async fn get_client(&self) -> ClientHandle {
        let (tx, rx) = channel(1);
        let tx_arc = Arc::new(tx);
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(client) = inner.free.pop() {
                drop(inner);
                return ClientHandle {
                    pool_inner: self.inner.clone(),
                    client: Some(client),
                };
            }
            inner.waiting.push_back(tx_arc.clone());
        }
        let mut waiter = ClientWaiter::new(self.inner.clone(), tx_arc, rx);
        waiter.recv().await
    }
}

pub struct ClientHandle {
    pool_inner: Arc<Mutex<ClientPoolInner>>,
    client: Option<Client>,
}

impl Drop for ClientHandle {
    fn drop(&mut self) {
        let client = take(&mut self.client).unwrap();
        let mut inner = self.pool_inner.lock().unwrap();
        inner.return_client(client);
    }
}

impl Deref for ClientHandle {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        self.client.as_ref().unwrap()
    }
}

struct ClientWaiter {
    pool: Arc<Mutex<ClientPoolInner>>,
    tx: Arc<Sender<Client>>,
    rx: Option<Receiver<Client>>,
}

impl ClientWaiter {
    pub fn new(
        pool: Arc<Mutex<ClientPoolInner>>,
        tx: Arc<Sender<Client>>,
        rx: Receiver<Client>,
    ) -> Self {
        ClientWaiter {
            pool: pool,
            tx: tx,
            rx: Some(rx),
        }
    }

    pub async fn recv(&mut self) -> ClientHandle {
        let client = self.rx.as_mut().unwrap().recv().await.unwrap();
        self.rx = None;
        ClientHandle {
            pool_inner: self.pool.clone(),
            client: Some(client),
        }
    }
}

impl Drop for ClientWaiter {
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

            // We might have been sent a client but never received
            // it, in which case we should free it.
            if let Ok(client) = rx.try_recv() {
                inner.return_client(client);
            }
        }
    }
}

struct ClientPoolInner {
    waiting: VecDeque<Arc<Sender<Client>>>,
    free: Vec<Client>,
}

impl ClientPoolInner {
    pub fn return_client(&mut self, client: Client) {
        if let Some(waiting) = self.waiting.pop_front() {
            // The buffer should never be full, and the
            // other side of the channel won't be dropped
            // unless it's removed from the waiting queue.
            waiting.try_send(client).unwrap();
        } else {
            self.free.push(client);
        }
    }
}
