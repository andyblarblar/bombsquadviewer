use std::io::Write;
use std::net::TcpListener;

use opencv::core::Mat;
use opencv::prelude::*;
use opencv::videoio::CAP_ANY;

fn main() -> anyhow::Result<()> {
    let server_ip: Vec<String> = std::env::args().take(2).collect();

    let mut video = opencv::videoio::VideoCapture::new(0, CAP_ANY)?;

    let stream = TcpListener::bind(server_ip[1].clone())?;

    loop {
        let (mut remote, addr) = stream.accept()?;
        println!("Connected to address: {:?}", addr);

        let mut is_first_trans = true;
        let mut buf = Mat::default();
        loop {
            if video.read(&mut buf)? {
                // Transmit size on first connection
                if is_first_trans {
                    let rows = buf.rows().to_be_bytes();
                    let cols = buf.cols().to_be_bytes();

                    remote.write_all(&rows)?;
                    remote.write_all(&cols)?;
                    is_first_trans = false;
                }

                let bytes = buf.data_bytes()?;

                if remote.write_all(bytes).is_err() {
                    println!("Connection broke, waiting for new connection...");
                    break;
                }
            } else {
                println!("Cannot Read!");
            }
        }
    }
}
