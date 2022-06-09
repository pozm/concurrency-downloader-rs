use std::{fmt::Debug, sync::{Arc, Mutex}};

use futures::{stream, StreamExt, Future, future::join_all};
use reqwest::Client;

const CONCURRENT_REQUESTS: usize = 16;


fn slice_at_idx<T: Clone>(v:Vec<T>,at:usize) -> Vec<Vec<T>>  {
    let mut nv = vec![];
    let mut tv = vec![];

    for (idx,value) in v.iter().enumerate() {
        if idx % at ==0 && !tv.is_empty() {
            nv.push(tv);
            tv = vec![value.clone()];
        } else {
            tv.push(value.clone());
        }
    }
    nv.push(tv);
    nv
}

pub async fn download_list<T:Into<String> + Debug + Clone, F:Fn(Vec<u8>) -> Fut,Fut: Future<Output=()>>(client: &Client, url: Vec<T>, on_complete: F) {
    let pog = slice_at_idx(url, 4);
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
                    let resp = client.get(url.into()).send().await?;
                    resp.bytes().await
                }
            })
            .buffer_unordered(CONCURRENT_REQUESTS);
    
        bodies
            .for_each(|b| async {
                match b {
                    Ok(b) => {
                        futures_rc.lock().unwrap().as_mut().unwrap().push(on_complete(b.to_vec()));
                    },
                    Err(e) => eprintln!("Got an error: {}", e),
                }
            })
            .await;
            let f = futures_rc.lock().unwrap().take().unwrap();
            join_all(f).await;
    }
}
