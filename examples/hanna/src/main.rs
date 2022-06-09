mod http_types;
use concurrency_download::{download_list};
use http_types::Root;
use reqwest::Client;
use tokio::{fs::{File, create_dir_all}, io::AsyncWriteExt};

#[tokio::main]
async fn main() {
    let client = Client::new();
    let hanna_bytes = include_bytes!("../hanna-out.json");
    let poggies:Root = serde_json::from_slice(hanna_bytes).unwrap();

    let urls = poggies.iter().map(|c|c.attachments.iter().map(|a|a.url.clone())).flatten().collect::<Vec<_>>();

    create_dir_all("./content").await.unwrap();



    download_list(&client, urls,|b| async move {
        let ext = match infer::get(&b) {
            Some(q) => q.extension(),
            None => "unknown",
        };
        println!("Got {} bytes with {}", b.len(),ext);
        let mut f= File::create(format!("./content/{}.{}",uuid::Uuid::new_v4(),ext)).await.unwrap();
        match f.write_all(&b).await {
            Ok(_) => println!("worky wrote"),
            Err(e) => println!("error writing {}",e),
        }
    },8usize).await;

}
