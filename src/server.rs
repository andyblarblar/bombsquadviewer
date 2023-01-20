use std::io::Write;
use std::net::{TcpListener, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
    let stream = UdpSocket::bind(server_ip[1].clone())?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;

    println!("Init!");

    loop {
        // Allow for blocking when waiting for new connection
        stream.set_read_timeout(None)?;

        // Wait for connection handshake initializer
        let mut syn_buf = [0u8; 2];
        let (_, addr) = stream.recv_from(&mut syn_buf)?;

        println!("SYN");

        stream.set_read_timeout(Some(Duration::from_secs(2)))?;

        // Acknowledge handshake and then wait for second ack
        if stream
            .connect(addr)
            .and_then(|_| stream.send(&syn_buf))
            .and_then(|_| stream.recv(&mut syn_buf))
            .is_err()
        {
            println!("Peer failed to complete handshake");
            continue;
        }

        println!("Connected to address: {:?}", addr);

        let mut is_first_trans = true;

        // Transmission loop
        while let Ok(frame) = rcv.recv() {
            // Transmit size on first connection
            if is_first_trans {
                let rows = (frame.height()).to_be_bytes();
                let cols = (frame.width()).to_be_bytes();

                if stream
                    .send(&rows[..])
                    .and_then(|_| stream.send(&cols[..]))
                    .is_err()
                {
                    break;
                }

                is_first_trans = false;
            }

            let bytes = frame.to_bytes();

            // Send frame data
            if stream.send(bytes).is_err() {
                println!("Socket err");
                break;
            }

            // Wait for ack (lets us know if client is connected, also serves as flow control)
            let mut ack_buf = [0u8; 1];
            if stream.recv(&mut ack_buf).is_err() {
                println!("Timed out waiting for ack");
                break;
            }
        }

        println!("Connection broke, waiting for new connection...");
    }
}
