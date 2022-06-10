use std::{
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex, RwLock}, iter::Map, collections::HashMap, borrow::BorrowMut,
};

use clap::Parser;
use concurrency_download::{download_list, download_list_stream};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};
use sha2::{Sha256, Sha512, Digest};


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
        create_dir_all(&out_dir).await.unwrap();
        let out_dir_d = &out_dir.display();
        let mut files : HashMap<&str, File> = HashMap::new();
        let mut files_m = Box::from(files);


        let on_complete = |b: Vec<u8>,url| async move {
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
        let on_partial = |b: Vec<u8>,url:String| async move {
            // println!("Got {} bytes ~ {}", b.len(),url);
            let mut h = sha2::Sha256::new();
            // let fmut = Rc::get_mut(&mut hb).unwrap();
            h.update(&url[..]);
            let hash_pog = format!("{:X}",h.finalize());
            // let files = files_m.lock().unwrap();
            match files_m.get_mut(&hash_pog[0..8]) {
                Some(f) => {f.write(&b).await;},
                None => {
                    let ext = match infer::get(&b) {
                        Some(q) => q.extension(),
                        None => "unknown",
                    };
                    let mut f = File::create(format!("{}/{}.{ext}",&out_dir_d,&hash_pog[0..8])).await.unwrap();
                    f.write(&b).await;
                    files_m.insert(&hash_pog[0..8], f);

                },
            };
            println!("{} | {} ",&hash_pog,b.len());
            vec![]
        };

        match cli.file {
            Some(file) => {
                println!("Downloading file: {}", file.display());
            }
            None => {
                println!("Downloading urls: {:?}", cli.urls);
                download_list_stream(&reqwest, cli.urls.unwrap(), on_complete,&on_partial, 8).await;
            }
        }
    }
}
