mod maple_aes;
use crate::maple_aes::MapleAES;

mod packet;
use crate::packet::Packet;

mod shanda;

mod maple_codec;
use crate::maple_codec::MapleCodec;

use deadpool_postgres::{Manager, Pool, Runtime};
use dotenv::dotenv;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::env;
use std::error::Error;
use std::net::SocketAddr;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use tokio_postgres::NoTls;
use tokio_stream::StreamExt;
use tokio_util::codec::Decoder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load environment variables from .env
    dotenv().ok();

    SimpleLogger::new()
        .with_module_level("tokio_util", LevelFilter::Debug)
        .with_module_level("mio", LevelFilter::Debug)
        .with_module_level("tokio_postgres", LevelFilter::Debug)
        .env()
        .init()
        .unwrap();

    let mut pg_config = tokio_postgres::Config::new();
    pg_config.user(&env::var("DATABASE_USER").unwrap());
    pg_config.password(&env::var("DATABASE_PASSWORD").unwrap());
    pg_config.dbname(&env::var("DATABASE_NAME").unwrap());
    pg_config.host("localhost");

    let manager = Manager::new(pg_config, NoTls);
    let pool = Pool::builder(manager).max_size(10).build().unwrap();

    let listener = TcpListener::bind("127.0.0.1:8484").await?;
    log::info!("Login server started on port 8484");

    loop {
        let (stream, addr) = listener.accept().await?;

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr).await {
                log::error!("An error occurred while starting the login server: {:?}", e);
            }
        });
    }
}

async fn handle_connection(mut stream: TcpStream, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    log::info!("Client connected to login server: {}", addr);

    let recv_iv: [u8; 4] = [0x46, 0x72, rand::random::<u8>(), 0x52];
    log::debug!("recv_iv: {:X?}", recv_iv);
    let recv_cipher = MapleAES::new(recv_iv, 83);

    let send_iv: [u8; 4] = [0x52, 0x30, 0x78, 0x61];
    log::debug!("send_iv: {:X?}", send_iv);
    let send_cipher = MapleAES::new(send_iv, 0xffff - 83);

    // write the initial unencrypted "hello" packet
    let handshake = login_handshake(recv_iv, send_iv);
    stream.write_all(&handshake.get_data()).await?;
    stream.flush().await?;

    let mut framed = MapleCodec::new(recv_cipher, send_cipher).framed(stream);

    while let Some(message) = framed.next().await {
        match message {
            Ok(mut packet) => {
                log::debug!("received packet: {}", packet);

                let op_code = packet.read_short();
                log::debug!("op_code: {} (0x{:X?})", op_code, op_code);

                if op_code >= 0x200 {
                    log::warn!(
                        "Potential malicious packet sent to login server from {}: 0x{:X?}",
                        addr,
                        op_code
                    );

                    break;
                }

                match op_code {
                    0x1 => handle_login_password(packet),
                    _ => log::warn!("Unhandled packet 0x{:X?}", op_code),
                }
            }
            Err(err) => println!("Socket closed with error: {:?}", err),
        }
    }

    println!("Socket received FIN packet and closed connection");

    Ok(())
}

fn login_handshake(iv_receive: [u8; 4], iv_send: [u8; 4]) -> Packet {
    let mut packet = Packet::new(18);
    packet.write_short(14); // packet length (0x0E)
    packet.write_short(83); // maple version (v83)
    packet.write_maple_string("1"); // maple patch version (1)
    packet.write_bytes(&iv_receive);
    packet.write_bytes(&iv_send);
    packet.write_byte(8); // locale
    packet
}

fn handle_login_password(mut packet: Packet) {
    let username = packet.read_maple_string();
    log::debug!("username: {}", username);

    let password = packet.read_maple_string();
    log::debug!("password: {}", password);

    packet.advance(6);

    let hwid = packet.read_bytes(4);
    log::debug!("hwid: {:02X?}", hwid);
}
