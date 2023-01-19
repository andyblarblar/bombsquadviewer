use std::io::Write;
use std::net::TcpListener;

use opencv::core::Mat;
use opencv::core::Size;
use opencv::imgproc::INTER_LINEAR;
use opencv::prelude::*;
use opencv::videoio::CAP_ANY;
use opencv::videoio::CAP_PROP_BUFFERSIZE;
use opencv::videoio::CAP_PROP_FORMAT;

fn main() -> anyhow::Result<()> {
    let server_ip: Vec<String> = std::env::args().take(2).collect();

    let mut video = opencv::videoio::VideoCapture::default()?;
    video.set(CAP_PROP_BUFFERSIZE, 1.0)?;
    video.set(CAP_PROP_FORMAT, -1.0)?;
    video.open(0, CAP_ANY)?;

    let stream = TcpListener::bind(server_ip[1].clone())?;

    println!("Init!");

    loop {
        let (mut remote, addr) = stream.accept()?;
        println!("Connected to address: {:?}", addr);

        let mut is_first_trans = true;
        let mut buf = Mat::default();
        loop {
            if video.read(&mut buf)? {
                let mut resize = Mat::default();

                // Downsample image to half size
                opencv::imgproc::resize(
                    &mut buf,
                    &mut resize,
                    Size::default(),
                    1.0,
                    1.0,
                    INTER_LINEAR,
                )?;

                // Transmit size on first connection
                if is_first_trans {
                    let rows = (resize.rows()).to_be_bytes();
                    let cols = (resize.cols()).to_be_bytes();

                    remote.write_all(&rows)?;
                    remote.write_all(&cols)?;
                    is_first_trans = false;
                }

                let bytes = resize.data_bytes()?;

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
