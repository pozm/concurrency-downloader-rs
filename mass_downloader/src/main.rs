use std::{
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use clap::Parser;
use concurrency_download::download_list;
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct ProgramArgs {
    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    file: Option<PathBuf>,
    #[clap(short, long, parse(from_os_str), value_name = "OUT_DIR")]
    output: Option<PathBuf>,
    #[clap(short, long, multiple_values = true)]
    urls: Option<Vec<String>>,
}

#[tokio::main]
async fn main() {
    let cli: ProgramArgs = ProgramArgs::parse();

    if cli.file.is_none() && cli.urls.is_none() {
        println!("No file or url specified");
        return;
    } else {
        let reqwest = reqwest::Client::new();
        let out_dir = cli.output.unwrap_or("./content".into());
        create_dir_all(out_dir).await.unwrap();

        let on_complete = |b: Vec<u8>| async move {
            let ext = match infer::get(&b) {
                Some(q) => q.extension(),
                None => "unknown",
            };
            println!("Got {} bytes with {}", b.len(), ext);
            let mut f = File::create(format!("./content/{}.{}", uuid::Uuid::new_v4(), ext))
                .await
                .unwrap();
            match f.write_all(&b).await {
                Ok(_) => println!("worky wrote"),
                Err(e) => println!("error writing {}", e),
            }
        };

        match cli.file {
            Some(file) => {
                println!("Downloading file: {}", file.display());
            }
            None => {
                println!("Downloading urls: {:?}", cli.urls);
                download_list(&reqwest, cli.urls.unwrap(), on_complete, 8).await;
            }
        }
    }
}
