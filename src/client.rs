use std::cmp::max;
use std::ffi::c_void;
use std::io::Read;
use std::net::TcpStream;
use std::time::Instant;

use opencv::core::CV_8UC3;
use opencv::highgui::{imshow, named_window, wait_key, WINDOW_FREERATIO};
use opencv::prelude::*;

fn main() -> anyhow::Result<()> {
    let server_ip: Vec<String> = std::env::args().take(2).collect();

    named_window("main", WINDOW_FREERATIO)?;

    let mut stream = TcpStream::connect(server_ip[1].clone())?;
    println!("Connected!");

    let mut frame_buffer: Box<[u8]> = Box::default();
    let mut is_first_trans = true;
    let mut dims: (i32, i32) = (0, 0);

    unsafe {
        loop {
            let start_time = Instant::now();

            // Get size info on first transmission
            if is_first_trans {
                let mut buf = [0u8; 4];

                // Get rows and cols for sizing things
                stream.read_exact(&mut buf)?;
                let rows = i32::from_be_bytes(buf);
                stream.read_exact(&mut buf)?;
                let cols = i32::from_be_bytes(buf);

                dims = (rows, cols);
                // Allocate frame buffer
                let vec = Vec::from_iter(std::iter::repeat(0).take((rows * cols * 3) as usize));
                frame_buffer = vec.into_boxed_slice();

                is_first_trans = false;
            }

            if stream.read_exact(&mut frame_buffer).is_err() {
                continue;
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
