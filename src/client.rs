use std::cmp::max;
use std::ffi::c_void;
use std::io::Read;
use std::net::{TcpStream, UdpSocket};
use std::time::{Duration, Instant};

use opencv::core::CV_8UC3;
use opencv::highgui::{imshow, named_window, wait_key, WINDOW_FREERATIO};
use opencv::prelude::*;

fn main() -> anyhow::Result<()> {
    let server_ip: Vec<String> = std::env::args().take(2).collect();

    if server_ip.len() < 2 {
        println!("Input IP:Port to connect to server");
        return Ok(());
    }

    named_window("main", WINDOW_FREERATIO)?;

    // Create socket
    let mut syn_buf = [0u8; 2];
    let socket = UdpSocket::bind("0.0.0.0:34343")?;
    println!("Bound socket to: {:?}", socket.local_addr()?);

    socket.set_write_timeout(Some(Duration::from_secs(2)))?;
    socket.set_read_timeout(Some(Duration::from_secs(2)))?;

    socket.connect(server_ip[1].clone())?;

    println!("Beginning handshake...");

    // Now we need to do our own three step handshake

    // Send syn
    socket.send(&syn_buf)?;
    println!("SYN");
    // Recv ack
    socket.recv(&mut syn_buf)?;
    println!("ACK");
    // Send ack
    socket.send(&syn_buf)?;
    println!("ACK");

    println!("Connected!");

    let mut frame_buffer: Box<[u8]> = Box::default();
    let mut is_first_trans = true;
    let mut dims: (i32, i32) = (0, 0);

    unsafe {
        'main_loop: loop {
            let start_time = Instant::now();

            // Get size info on first transmission
            if is_first_trans {
                let mut buf = [0u8; 4];

                // Get rows and cols for sizing things
                socket.recv(&mut buf)?;
                let rows = i32::from_be_bytes(buf);
                socket.recv(&mut buf)?;
                let cols = i32::from_be_bytes(buf);

                // Ack
                socket.send(&syn_buf)?;

                dims = (rows, cols);
                // Allocate frame buffer
                let vec = Vec::from_iter(std::iter::repeat(0).take((rows * cols * 3) as usize));
                frame_buffer = vec.into_boxed_slice();

                is_first_trans = false;
            }

            // Get next frame
            {
                let mut bytes_read = 0;

                // Segment reconstruction is synchronised to avoid out of order segments
                while bytes_read < frame_buffer.len() {
                    let res = socket.recv(&mut frame_buffer[bytes_read..]);
                    match res {
                        Err(_) => continue 'main_loop,
                        Ok(count) => bytes_read += count,
                    }

                    // Ack
                    socket.send(&syn_buf)?;

                    //println!("Bytes read: {}", bytes_read);
                }
            }

            let mat_buf = Mat::new_rows_cols_with_data(
                dims.0,
                dims.1,
                CV_8UC3,
                frame_buffer.as_mut_ptr() as *mut c_void,
                0,
            )?;
            imshow("main", &mat_buf)?;

            if wait_key(1)? == 0x81 {
                break;
            }

            // Print fps
            println!(
                "FPS: {}",
                (1.0 / max((Instant::now() - start_time).as_millis(), 1) as f64) * 1000.0
            );
        }
    }

    Ok(())
}
