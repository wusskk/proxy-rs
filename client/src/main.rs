use anyhow::{anyhow, Result};
use tokio::{
    io::{AsyncWriteExt, ErrorKind},
    net::{TcpListener, TcpStream},
    time::{sleep, Duration},
};

use log::{error, info};
use proxy::message::Message;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let listener = TcpListener::bind("127.0.0.1:10800").await?;
    let mut id: usize = 0;
    loop {
        let (client, _) = listener.accept().await?;
        // 为每一条连接都生成一个新的任务，
        // `client` 的所有权将被移动到新的任务中，并在那里进行处理
        tokio::spawn(async move {
            if let Err(e) = handle_connection(client, id).await {
                error!("{}: error while handling stream:\n{}", id, e);
            };
            info!("[Thread {id}] finished!");
        });

        if id < 999 {
            id = id + 1;
        } else {
            id = 0;
        }
    }
}

async fn handle_connection(mut client: TcpStream, id: usize) -> Result<()> {
    let mut server = TcpStream::connect("127.0.0.1:10801").await?;

    let mut nbytes_client: usize = 0;
    let mut nbytes_server: usize = 0;

    let mut client_buf: [u8; 512] = [0; 512];
    let mut server_buf: [u8; 512] = [0; 512];

    loop {
        if nbytes_client == 0 {
            nbytes_client = match client.try_read(&mut client_buf[0..496]) {
                Ok(0) => {
                    info!("{}: no data from client", id);
                    return Ok(());
                }
                Ok(n) => n,
                Err(e) if e.kind() == ErrorKind::WouldBlock => 0,
                Err(e) => return Err(anyhow!("{}: error while read from client:\n{}", id, e)),
            }
        }

        if nbytes_client > 0 {
            info!("{}: received {} bytes data from client", id, nbytes_client);
            Message::from_body_to_vec(id, &mut client_buf[..512], nbytes_client)?;

            match server.write(&client_buf).await {
                Ok(0) => return Ok(()),
                Ok(n) if n == 512 => nbytes_client = 0,
                Ok(_) => return Ok(()),
                Err(e) => return Err(anyhow!("{}: error while write to server:\n{}", id, e)),
            }
        }

        if nbytes_server == 0 {
            nbytes_server = match server.try_read(&mut server_buf[0..512]) {
                Ok(0) => {
                    info!("{}: no data from client", id);
                    return Ok(());
                }
                Ok(n) => n,
                Err(e) if e.kind() == ErrorKind::WouldBlock => 0,
                Err(e) => return Err(anyhow!("{}: error while read from server:\n{}", id, e)),
            }
        }
        if nbytes_server > 0 {
            info!("{}: received {} bytes data from server", id, nbytes_server);
            let s_len = Message::len_fromn_u8(&server_buf);
            match client.write(&server_buf[..s_len]).await {
                Ok(0) => return Ok(()),
                Ok(n) if n == s_len => nbytes_server = 0,
                Ok(_) => return Ok(()),
                Err(e) => return Err(anyhow!("{}: error while write to client:\n{}", id, e)),
            }
        }
        sleep(Duration::from_millis(1)).await;
    }
}
