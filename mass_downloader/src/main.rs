use std::{
    path::PathBuf,
    rc::Rc,
    fs::{File},
    io::{prelude::*},
    sync::{Arc, Mutex, RwLock}, iter::Map, collections::HashMap, borrow::BorrowMut,
};

use clap::Parser;
use concurrency_download::{download_list, download_list_stream};
use indicatif::{MultiProgress, ProgressStyle, ProgressBar};
use tokio::{
    fs::{create_dir_all, /* File, */ OpenOptions},
    io::{AsyncWriteExt, AsyncReadExt, BufReader, AsyncBufReadExt},
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
    #[clap(short, long,default_value = "8")]
    concurrent: u16
}

// lol fuck you fnonce
lazy_static::lazy_static!{
    static ref HTOFILE :  Arc<RwLock<HashMap<String, (File,u16)>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref MULTIPROG : Arc<RwLock<MultiProgress>> = Arc::new(RwLock::new(MultiProgress::new()));
    static ref MPBS : Arc<RwLock<Vec<(ProgressBar,bool)>>> = Arc::new(RwLock::new(vec![]));
    static ref MASTERPB : Arc<RwLock<Option<ProgressBar>>> = Arc::new(RwLock::new(None));
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

        for i in 0..cli.concurrent {
            let pb = ProgressBar::new_spinner();
            let pb = MULTIPROG.write().unwrap().insert_from_back(0, pb);
            MPBS.write().unwrap().push((pb,false))
        }
        let master = ProgressBar::new_spinner();
        let master = MULTIPROG.write().unwrap().insert_from_back(0, master);
        let master_sty = ProgressStyle::with_template(
            "[{elapsed_precise}] {wide_bar:.magenta.bold/magenta} {pos:>1}/{len:1} {msg} ({eta})",
        )
        .unwrap()
        .progress_chars("█▓▒░  ");
        // let mut files : HashMap<String, File> = HashMap::new();
        // let mut files_m =  Arc::new(Mutex::new(files));

        // let m = Arc::new(RwLock::new(MultiProgress::new()));
        // let sty = ProgressStyle::with_template(
        //     "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        // )
        // .unwrap()
        // .progress_chars("##-");


        let on_complete = |b: Vec<u8>,url:String| async move {
            let mut h = sha2::Sha256::new();
            h.update(&url[..]);
            let hash_pog = format!("{:X}",h.finalize());
            // println!("finish {}", &hash_pog[0..8]);
            let mut fmut = HTOFILE.write().expect("unable to unlock");
            if let Some(f) = fmut.get_mut(&hash_pog[0..8]) {
                f.0.flush().expect("unable to flush");
                // let sty = ProgressStyle::with_template(
                //     " [{elapsed_precise}] {wide_bar:40.cyan/blue} {bytes}/{total_bytes} {msg}",
                // )
                // .unwrap()
                // .progress_chars("#>-");
                // f.1.set_style(sty);
                // f.1.finish_with_message((&hash_pog[0..8]).to_string());
                // MULTIPROG.write().expect("haa").remove(&f.1);
                let mut pbbbs =MPBS.write().unwrap();
                pbbbs.get_mut(f.1 as usize).unwrap().1 = false;
                // MULTIPROG.write().unwrap().println(&format!("{} has completed",&hash_pog[0..8])).expect("not pogger");

                // MASTERPB.write().unwrap().as_ref().unwrap().inc(1);

                fmut.remove(&hash_pog[0..8]);
            } else {
                // println!("{} not open??", &hash_pog[0..8]);
            }
            // println!("hi")
        };
        let on_partial = |b: Vec<u8>,url:String,size:u64,total_size:u64| async move {
            let mut h = sha2::Sha256::new();
            h.update(&url[..]);
            let hash_pog = format!("{:X}",h.finalize());
            let mut fmut = HTOFILE.write().expect("unable to unlock");
            // let keys=  fmut.len();
            let key = fmut.get_mut(&hash_pog[0..8]);
            // println!("d {} {} - {size}/{total_size} ({}) | {:?}",&hash_pog[0..8], b.len(),keys,key);
            match key {
                Some(f) => {
                    // println!("\tprw {} {}/{}", &hash_pog[0..8], size, total_size);
                    f.0.write(&b).expect("h");
                    // println!("\tpow {}", &hash_pog[0..8]);
                    let mut pbbbs =MPBS.write().unwrap();
                    pbbbs.get_mut(f.1 as usize).unwrap().0.set_position(size);
                    if size >= total_size {
                        MULTIPROG.write().unwrap().println(&format!("{} has completed",&hash_pog[0..8])).expect("not pogger");
                        MASTERPB.write().unwrap().as_ref().unwrap().inc(1);
                    }
                },
                None => {
                    // println!("\tinf {}", &hash_pog[0..8]);
                    let ext = match infer::get(&b) {
                        Some(q) => q.extension(),
                        None => "unknown",
                    };
                    // println!("\tafinf {} {}", &hash_pog[0..8], ext);
                    // file
                    let mut f = std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(format!("{}/{}.{ext}",&out_dir_d,&hash_pog[0..8])).expect("unable to open file");
                    // println!("\to {}", &hash_pog[0..8]);
                    f.write(&b).expect("bruh");
                    // println!("\tc {}", &hash_pog[0..8]);
                    // let last = fmut.
                    // LAST
                    let sty = ProgressStyle::with_template(
                        "{spinner:.magenta} [{elapsed_precise}] {wide_bar:.magenta/magenta.dim} {bytes}/{total_bytes} {msg} ({eta})",
                    )
                    .unwrap()
                    .progress_chars("#>-");
                    // let p = ProgressBar::new(total_size);
                    let mut pbbbs =MPBS.write().unwrap();
                    let p = pbbbs.iter_mut().enumerate().find(|(_i,x)| x.1 == false).unwrap();
                    let i = p.0;
                    let p = p.1;
                    p.0.set_style(sty);
                    p.0.set_length(total_size);
                    p.0.set_position(size);
                    p.0.set_message((&hash_pog[0..8]).to_string());
                    p.1 = true;
                    // MULTIPROG.write().expect("haa")
                    fmut.insert((&hash_pog[0..8]).to_string(), (f,i as u16));
                    // println!("\ti {}", &hash_pog[0..8]);
                    

                },
            };
            // println!("x {}", &hash_pog[0..8]);
            // println!("{} | {} ",&hash_pog[0..8],b.len());
            vec![]
        };

        master.set_message("total");
        match cli.file {
            Some(file) => {
                println!("Downloading file: {}", file.display());
                let mut urls = vec![];
                let f = std::fs::OpenOptions::new().read(true).open(file).unwrap();
                let reader = std::io::BufReader::new(f);
                let mut lines = reader.lines();
                for l in lines {
                    urls.push(l.unwrap());
                }
                println!("{:#?} urls", urls.len());
                master.set_style(master_sty);
                master.set_length(urls.len() as u64);
                *MASTERPB.write().unwrap() = Some(master);
                download_list_stream(&reqwest, urls, on_complete,&on_partial, cli.concurrent as usize).await;
            }
            None => {
                println!("Downloading urls: {:?}", cli.urls);
                master.set_style(master_sty);
                master.set_length(cli.urls.as_ref().unwrap().len() as u64);
                *MASTERPB.write().unwrap() = Some(master);
                download_list_stream(&reqwest, cli.urls.unwrap(), on_complete,&on_partial, cli.concurrent as usize).await;
            }
        }

        println!()
    }
}
