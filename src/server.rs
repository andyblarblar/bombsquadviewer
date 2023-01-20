use std::io::Write;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use uvc::Frame;

fn main() -> anyhow::Result<()> {
    let server_ip: Vec<String> = std::env::args().take(2).collect();

    if server_ip.len() < 2 {
        println!("Either input an IP:Port to host server, or enter 'format' to view formats");
        return Ok(());
    }

    // Find webcam
    let ctx = uvc::Context::new().expect("Could not get context");
    let dev = ctx
        .find_device(None, None, None)
        .expect("Could not find device");
    let dev = dev.open().expect("Could not open device");

    // List formats if requested
    if server_ip[1] == "format" {
        for format in dev.supported_formats() {
            println!("With format type: {:?}", format.subtype(),);

            for format in format.supported_formats() {
                println!(
                    "{}x{} fps: {:?}",
                    format.width(),
                    format.height(),
                    format
                        .intervals_duration()
                        .iter()
                        .map(|s| (1.0 / s.as_millis() as f64 * 1000.0).round())
                        .collect::<Vec<f64>>()
                );
            }
            println!();
        }
        return Ok(());
    }

    // Setup stream
    let format = uvc::StreamFormat {
        width: 640,
        height: 480,
        fps: 30,
        format: uvc::FrameFormat::Uncompressed,
    };
    let mut streamh = dev
        .get_stream_handle_with_format(format)
        .expect("Could not open a stream with this format");

    // Synchronisation tools
    let (send, rcv) = std::sync::mpsc::sync_channel::<Arc<Frame>>(30);
    let send = Arc::new(Mutex::new(send));

    // Spawn webcam in the background, reading into buffer
    let _streamh = streamh
        .start_stream(
            |frame, sender| {
                let sender = sender.lock().unwrap();
                sender
                    .send(Arc::new(
                        frame.to_bgr().expect("Format does not support BGR"),
                    ))
                    .unwrap();
            },
            send,
        )
        .unwrap();

    // Start server
    let stream = TcpListener::bind(server_ip[1].clone())?;

    println!("Init!");

    loop {
        // Wait for connection
        let (mut remote, addr) = stream.accept()?;

        println!("Connected to address: {:?}", addr);

        let mut is_first_trans = true;

        while let Ok(frame) = rcv.recv() {
            // Transmit size on first connection
            if is_first_trans {
                let rows = (frame.height()).to_be_bytes();
                let cols = (frame.width()).to_be_bytes();

                if remote
                    .write_all(&rows)
                    .and_then(|_| remote.write_all(&cols))
                    .is_err()
                {
                    break;
                }

                is_first_trans = false;
            }

            let bytes = frame.to_bytes();

            if remote.write_all(bytes).is_err() {
                break;
            }
        }

        println!("Connection broke, waiting for new connection...");
    }
}
