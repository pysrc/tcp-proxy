use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    listen: u16,
    forward: Vec<String>,
}

async fn handle(pi: Person) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", pi.listen))
        .await
        .unwrap();
    println!(":{} -> {:?}", pi.listen, &pi.forward);
    let lenp = pi.forward.len();
    let mut ki = 0;
    let mut next = || {
        if lenp == 1 {
            0
        } else {
            ki += 1;
            ki = if ki >= lenp { 0 } else { ki };
            return ki;
        }
    };
    while let Ok((mut inbound, _)) = listener.accept().await {
        let forr = pi.forward.clone();
        let hst = forr[next()].clone();
        tokio::spawn(async move {
            let mut outbound = match TcpStream::connect(hst).await {
                Ok(outbound) => outbound,
                Err(_) => {
                    return;
                }
            };
            let (mut ri, mut wi) = inbound.split();
            let (mut ro, mut wo) = outbound.split();
            let client_to_server = async {
                if let Err(_) = io::copy(&mut ri, &mut wo).await {}
                if let Err(_) = wo.shutdown().await {}
            };

            let server_to_client = async {
                if let Err(_) = io::copy(&mut ro, &mut wi).await {}
                if let Err(_) = wi.shutdown().await {};
            };
            tokio::join!(client_to_server, server_to_client);
        });
    }
}

fn main() {
    let f = File::open("config.json");
    let cfg = match f {
        Ok(mut file) => {
            let mut config = String::new();
            file.read_to_string(&mut config).unwrap();
            let p: Vec<Person> = serde_json::from_str(&config).unwrap();
            p
        }
        Err(e) => {
            panic!("{}", e);
        }
    };
    // start tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut joins: Vec<JoinHandle<()>> = Vec::with_capacity(cfg.len());
            for pi in cfg {
                let jh = tokio::spawn(async move {
                    handle(pi).await;
                });
                joins.push(jh);
            }
            for ji in joins {
                tokio::try_join!(ji).unwrap();
            }
        });
}
