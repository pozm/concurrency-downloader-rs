use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use futures::{future::join_all, stream, Future, StreamExt};
use reqwest::{Client, Request};

// const CONCURRENT_REQUESTS: usize = 16;

fn slice_at_idx<T: Clone>(v: Vec<T>, at: usize) -> Vec<Vec<T>> {
    let mut nv = vec![];
    let mut tv = vec![];

    for (idx, value) in v.iter().enumerate() {
        if idx % at == 0 && !tv.is_empty() {
            nv.push(tv);
            tv = vec![value.clone()];
        } else {
            tv.push(value.clone());
        }
    }
    nv.push(tv);
    nv
}

pub async fn download_list<
    T: Into<String> + Debug + Clone,
    F: Fn(Vec<u8>) -> Fut,
    Fut: Future<Output = ()>,
>(
    client: &Client,
    url: Vec<T>,
    on_complete: F,
    concurrent_max: usize,
) {
    let pog = slice_at_idx(url, concurrent_max);
    for urls in pog {
        if urls.is_empty() {
            continue;
        }
        let futures: Option<Vec<Fut>> = Some(vec![]);
        let futures_rc = Arc::new(Mutex::new(futures));

        let bodies = stream::iter(urls)
            .map(|url| {
                let client = &client;
                async move {
                    let resp = client.get(url.into()).send().await.unwrap();
                    resp.bytes().await
                }
            })
            .buffer_unordered(concurrent_max);

        bodies
            .for_each(|b| async {
                match b {
                    Ok(b) => {
                        futures_rc
                            .lock()
                            .unwrap()
                            .as_mut()
                            .unwrap()
                            .push(on_complete(b.to_vec()));
                    }
                    Err(e) => eprintln!("Got an error: {}", e),
                }
            })
            .await;
        let f = futures_rc.lock().unwrap().take().unwrap();
        join_all(f).await;
    }
}
#[cfg(feature = "stream")]
pub async fn download_list_stream<
    T: Into<String> + Debug + Clone,
    F: Fn(Vec<u8>,String) -> Fut,
    F2: Fn(Vec<u8>,String,u64,u64) -> Fut2,
    Fut: Future<Output = ()>,
    Fut2: Future<Output = Vec<u8>>,
>(
    client: &Client,
    url: Vec<T>,
    on_complete: F,
    on_partial: &F2,
    concurrent_max: usize,
) {
    let pog = slice_at_idx(url, concurrent_max);
    for urls in pog {
        if urls.is_empty() {
            continue;
        }
        let futures: Option<Vec<Fut>> = Some(vec![]);
        let futures_rc = Arc::new(Mutex::new(futures));

        let bodies = stream::iter(urls)
            .map(|url| {
                let client = &client;
                async move {
                    let ur = url.into();
                    let resp = client.get(&ur).send().await.unwrap();
                    let total_size = resp.content_length().unwrap_or(0);
                    let mut s = resp.bytes_stream();
                    let mut rep = vec![];
                    let mut current_size = 0u64;
                    while let Some(item) = s.next().await {
                        current_size+=item.as_ref().unwrap().len() as u64;
                        rep.append(&mut on_partial(item.unwrap().to_vec(),ur.clone(),current_size,total_size).await);
                    }
                    (rep,ur)
                }
            })
            .buffer_unordered(concurrent_max);
        bodies
        .for_each(|b| async {
            futures_rc
                        .lock()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .push(on_complete(b.0,b.1));
        })
        .await;
        let f = futures_rc.lock().unwrap().take().unwrap();
        join_all(f).await;
    }
}
