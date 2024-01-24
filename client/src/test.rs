use std::{collections::HashMap, sync::Mutex};

use anyhow::{anyhow, Result};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
    sync::{
        mpsc,
        mpsc::{Receiver, Sender},
    },
    try_join,
};

use lazy_static::lazy_static;
use log::error;
use proxy::message::Message;

lazy_static! {
    static ref TX_CHILD: Mutex<HashMap<usize, Sender<Message>>> = Mutex::new(HashMap::new());
}



#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let proxy_server = match TcpStream::connect("127.0.0.1:10801").await {
        Ok(s) => s,
        Err(e) => return Err(anyhow!("error while connect to proxy server:\n{}", e)),
    };

    let (ps_read, ps_write) = proxy_server.into_split();

    let (tx_send, rx_send) = mpsc::channel(4096);
    //let (tx_rece, rx_rece) = mpsc::channel(100);

    tokio::spawn(async move { send_message(rx_send, ps_write).await });
    tokio::spawn(async move { receive_message(ps_read).await });

    let listener: TcpListener = TcpListener::bind("127.0.0.1:10800").await?;
    let mut id: usize = 0;

    loop {
        let (client, _) = listener.accept().await?;
        // 为每一条连接都生成一个新的任务，
        // `client` 的所有权将被移动到新的任务中，并在那里进行处理
        let tx_send_child = tx_send.clone();
        // let tx_rece_child = tx_rece.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(client, id, tx_send_child).await {
                error!("error while handling stream:\n{}", e);
            };
        });

        if id < 999 {
            id = id + 1;
        } else {
            id = 0;
        }
    }
}

async fn handle_connection(
    mut client: TcpStream,
    id: usize,
    tx_send: Sender<Message>,
) -> Result<()> {
    let (mut reader, mut writer) = client.split();
    let (tx_rece, mut rx_rece) = mpsc::channel(4096);
    let mut send_buf: Vec<u8> = Vec::new();
    let mut rece_buf: Vec<u8>;
    TX_CHILD.lock().unwrap().insert(id, tx_rece).unwrap();

    loop {
        let send_msg = Message {
            id: id,
            body: serde_json::from_slice(&send_buf).unwrap(),
        };
        tx_send.send(send_msg).await.unwrap();

        let rece_msg = rx_rece.recv().await.unwrap();
        rece_buf = serde_json::to_vec(&rece_msg.body).unwrap();
        let mut tmp_rece_buf = rece_buf.as_slice();
        try_join!(
            io::copy(&mut reader, &mut send_buf),
            io::copy(&mut tmp_rece_buf, &mut writer)
        )
        .unwrap();
    }

    //Ok(())
}

async fn send_message(mut rx: Receiver<Message>, mut writer: OwnedWriteHalf) -> Result<()> {
    loop {
        let message = rx.recv().await.unwrap();
        let m_u8 = serde_json::to_vec(&message).unwrap();
        writer.write(&m_u8[..]).await.unwrap();
    }
}

async fn receive_message(mut reader: OwnedReadHalf) -> Result<()> {
    let mut buf = [0; 4096];
    loop {
        let n = match reader.read(&mut buf).await {
            Ok(n) => n,
            Err(e) => return Err(anyhow!("error while client reading from server:\n{e}")),
        };
        let message: Message = match serde_json::from_slice(&buf[..n]) {
            Ok(m) => m,
            Err(e) => return Err(anyhow!("error while u8 change to message:\n{e}")),
        };
        let tx = TX_CHILD.lock().unwrap().get(&message.id).unwrap().clone();
        match tx.send(message).await {
            Ok(()) => (),
            Err(e) => {
                return Err(anyhow!(
                    "error while send message from client to child thread:\n{e}"
                ))
            }
        };
    }
}
