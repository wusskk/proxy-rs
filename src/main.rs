use anyhow::{anyhow, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    // join,
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpListener, TcpStream,
    },
    time::{sleep, Duration},
};

use log::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let listener = TcpListener::bind("127.0.0.1:10800").await?;
    let mut id = 0;
    loop {
        let (client, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_connection(client, id).await {
                error!("error while handling stream:\n{}", e);
            };
        });
        id = id + 1;
    }
}

async fn handle_connection(mut client: TcpStream, id: usize) -> Result<()> {
    let mut buf = [0; 4096];

    let n = match client.read(&mut buf).await {
        Ok(0) => return Err(anyhow!("no data in the first request form client!")),
        Ok(n) => {
            info!("{}: received {n} bytes from the request", id);
            n
        }
        Err(e) => return Err(anyhow!("error while read request:\n{}", e)),
    };

    let req_str = std::str::from_utf8(&buf[..n]).unwrap();
    let mut req = req_str.split_whitespace();
    let method = req.nth(0).unwrap();
    let host = req.nth(3).unwrap();

    let mut server: TcpStream;

    if method == "CONNECT" {
        info!("{}: connect to {}", id, host);

        server = match TcpStream::connect(host).await {
            Ok(s) => s,
            Err(e) => return Err(anyhow!("error while connect to server:\n{}", e)),
        };

        match client
            .write(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await
        {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("error while write data to client:\n{}", e)),
        };
    } else {
        let host = format!("{}:80", host);
        info!("{}: connect to {}", id, host);

        server = match TcpStream::connect(host).await {
            Ok(s) => s,
            Err(e) => return Err(anyhow!("error while connect to server:\n{}", e)),
        };

        match server.write(&buf).await {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("error while write data to client:\n{}", e)),
        };
    }

    let (mut client_reader, mut client_writer) = client.split();
    let (mut server_reader, mut server_writer) = server.split();

    let mut nbytes_client: usize = 0;
    let mut nbytes_server: usize = 0;

    let mut client_buf: [u8; 4096] = [0; 4096];
    let mut server_buf: [u8; 4096] = [0; 4096];
    //(nbytes_client, client_reader, server_writer, client_buf),
    // (nbytes_server, server_reader, client_writer, server_buf),
    loop {
        // let (a, b) = join!(
        //     client_to_server(client_reader, server_writer, nbytes_client, client_buf, id),
        //     server_to_client(server_reader, client_writer, nbytes_server, server_buf, id)
        // );

        // (nbytes_client, client_reader, server_writer, client_buf) = match a {
        //     Ok(s) => s,
        //     Err(e) => return Err(anyhow!("error while client to server:\n{}", e)),
        // };

        // (nbytes_server, server_reader, client_writer, server_buf) = match b {
        //     Ok(s) => s,
        //     Err(e) => return Err(anyhow!("error while server to client:\n{}", e)),
        // };

        (nbytes_client, client_reader, server_writer, client_buf) =
            client_to_server(client_reader, server_writer, nbytes_client, client_buf, id)
                .await
                .unwrap();
        (nbytes_server, server_reader, client_writer, server_buf) =
            server_to_client(server_reader, client_writer, nbytes_server, server_buf, id)
                .await
                .unwrap();

        if nbytes_client == 5000 || nbytes_server == 5000 {
            info!("Thread {id} finish!");
            break;
        }
        sleep(Duration::from_millis(1)).await;
    }

    Ok(())

async fn client_to_server<'a>(
    mut client: ReadHalf<'a>,
    mut server: WriteHalf<'a>,
    mut nbytes_client: usize,
    mut client_buf: [u8; 4096],
    id: usize,
) -> Result<(usize, ReadHalf<'a>, WriteHalf<'a>, [u8; 4096])> {
    if nbytes_client == 0 {
        nbytes_client = match client.read(&mut client_buf).await {
            Ok(0) => {
                info!("{}: no data from client", id);
                return Ok((5000, client, server, client_buf));
            }
            Ok(n) => n,
            Err(e) => return Err(anyhow!("error while read from client:\n{}", e)),
        }
    }
    info!("{}: received {} data from client:", id, nbytes_client);

    if nbytes_client > 0 {
        match server.write(&mut client_buf[..nbytes_client]).await {
            Ok(0) => {
                info!("{id}: write to sever 0 bytes");
                return Ok((5000, client, server, client_buf));
            }
            Ok(n) if n == nbytes_client => nbytes_client = 0,
            Ok(_) => {
                info!("{id}: write to sever more than n bytes");
                return Ok((5000, client, server, client_buf));
            }
            Err(e) => return Err(anyhow!("error while write to server:\n{}", e)),
        }
    }

    Ok((nbytes_client, client, server, client_buf))
}

async fn server_to_client<'a>(
    mut server: ReadHalf<'a>,
    mut client: WriteHalf<'a>,
    mut nbytes_server: usize,
    mut server_buf: [u8; 4096],
    id: usize,
) -> Result<(usize, ReadHalf<'a>, WriteHalf<'a>, [u8; 4096])> {
    if nbytes_server == 0 {
        nbytes_server = match server.read(&mut server_buf).await {
            Ok(0) => {
                info!("{}: no data from client", id);
                return Ok((5000, server, client, server_buf));
            }
            Ok(n) => n,
            Err(e) => return Err(anyhow!("error while read from server:\n{}", e)),
        }
    }
    info!("{}: received {} data from server:", id, nbytes_server);

    if nbytes_server > 0 {
        match client.write(&mut server_buf[..nbytes_server]).await {
            Ok(0) => {
                info!("{id}: write to client 0 bytes");
                return Ok((5000, server, client, server_buf));
            }
            Ok(n) if n == nbytes_server => nbytes_server = 0,
            Ok(_) => {
                info!("{id}: write to client more than n bytes");
                return Ok((5000, server, client, server_buf));
            }
            Err(e) => return Err(anyhow!("error while write to client:\n{}", e)),
        }
    }
    Ok((nbytes_server, server, client, server_buf))
}