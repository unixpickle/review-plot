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
        let (tx, mut rx) = channel(1);
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(client) = inner.free.pop() {
                drop(inner);
                return ClientHandle {
                    pool_inner: self.inner.clone(),
                    client: Some(client),
                };
            }
            inner.waiting.push_back(tx);
        }
        // TODO: if our async future gets dropped, we should
        // remove ourselves from the queue and then make sure
        // not to leak a client.
        let client = rx.recv().await.unwrap();
        ClientHandle {
            pool_inner: self.inner.clone(),
            client: Some(client),
        }
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
        if let Some(waiting) = inner.waiting.pop_front() {
            // The buffer should never be full, and the
            // other side of the channel won't be dropped
            // unless it's removed from the waiting queue.
            waiting.try_send(client).unwrap();
        } else {
            inner.free.push(client);
        }
    }
}

impl Deref for ClientHandle {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        self.client.as_ref().unwrap()
    }
}

struct ClientPoolInner {
    waiting: VecDeque<Sender<Client>>,
    free: Vec<Client>,
}
