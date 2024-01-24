use anyhow::{anyhow, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ErrorKind},
    net::{TcpListener, TcpStream},
    time::{sleep, Duration},
};

use log::{error, info};
use proxy::message::Message;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let listener = TcpListener::bind("127.0.0.1:10801").await?;
    loop {
        let (client, _) = listener.accept().await?;
        // 为每一条连接都生成一个新的任务，
        // `client` 的所有权将被移动到新的任务中，并在那里进行处理
        tokio::spawn(async move {
            if let Err(e) = handle_connection(client).await {
                error!("error while handling stream:\n{}", e);
            };
        });
    }
}

async fn handle_connection(mut client: TcpStream) -> Result<()> {
    let mut buf = [0; 512];

    let _n = match client.read(&mut buf).await {
        Ok(0) => return Err(anyhow!("no data in the first request form client!")),
        Ok(n) => {
            info!("received {n} bytes from the request");
            n
        }
        Err(e) => return Err(anyhow!("error while read request:\n{}", e)),
    };

    let len = Message::len_fromn_u8(&buf);
    let id = Message::id_fromn_u8(&buf);
    info!("{id}: received {len} bytes first from the client");
    let req = std::str::from_utf8(&buf[..len]).unwrap();
    let host = req.split_whitespace().nth(4).unwrap();

    let mut server: TcpStream;

    if req.starts_with("CONNECT") {
        info!("{}: connect to {}", id, host);

        server = match TcpStream::connect(host).await {
            Ok(s) => s,
            Err(e) => return Err(anyhow!("error while connect to server:\n{}", e)),
        };
        let sync_msg = Message::from_str_to_vec(id, "HTTP/1.1 200 Connection Established\r\n\r\n");

        match client.write(&sync_msg).await {
            Ok(_) => info!("{id}: sync msg is sending to client!"),
            Err(e) => return Err(anyhow!("error while write data to client:\n{}", e)),
        };
    } else {
        let host = format!("{}:80", host);
        info!("{}: connect to {}", id, host);

        server = match TcpStream::connect(host).await {
            Ok(s) => s,
            Err(e) => return Err(anyhow!("error while connect to server:\n{}", e)),
        };

        match server.write(&buf[..len]).await {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("error while write data to client:\n{}", e)),
        };
    }

    let mut nbytes_client: usize = 0;
    let mut nbytes_server: usize = 0;

    let mut client_buf: [u8; 512] = [0; 512];
    let mut server_buf: [u8; 512] = [0; 512];

    loop {
        if nbytes_client == 0 {
            nbytes_client = match client.try_read(&mut client_buf) {
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
            let c_len = Message::len_fromn_u8(&client_buf);
            match server.write(&client_buf[..c_len]).await {
                Ok(0) => return Ok(()),
                Ok(n) if n == c_len => nbytes_client = 0,
                Ok(_) => return Ok(()),
                Err(e) => return Err(anyhow!("{}: error while write to server:\n{}", id, e)),
            }
        }

        if nbytes_server == 0 {
            nbytes_server = match server.try_read(&mut server_buf) {
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
            Message::from_body_to_vec(id, &mut server_buf[..512], nbytes_server)?;

            match client.write(&server_buf[..512]).await {
                Ok(0) => return Ok(()),
                Ok(n) if n == 512 => nbytes_server = 0,
                Ok(_) => return Ok(()),
                Err(e) => return Err(anyhow!("{}: error while write to client:\n{}", id, e)),
            }
        }
        sleep(Duration::from_millis(1)).await;
    }
}
