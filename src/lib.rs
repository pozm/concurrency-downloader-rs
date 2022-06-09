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
    Fb: Fn(&Client,&T) -> reqwest::Result<Bytes>
>(
    client: &Client,
    url: Vec<T>,
    on_complete: F,
    make_request:Option<Fb>,
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
                    let g = match make_request {
                        Some(f) => f(client,&url).await.into(),
                        None => {
                            let resp = client.get(url.into()).send().await.unwrap();
                            let g = resp.bytes().await;
                            g
                        },
                    };
                    g
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
