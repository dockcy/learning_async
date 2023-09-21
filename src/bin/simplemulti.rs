use std::io;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("localhost:30000").unwrap();

    let (connection, _) = listener.accept().unwrap();

    std::thread::spawn(|| {
        if let Err(e) = handle_connection(connection) {
            println!("failed to handle the connection : {e} co");
        }
    });
}

fn handle_connection(mut connection: TcpStream) -> io::Result<()> {
    get_request(&mut connection)?;

    send_response(&mut connection)?;

    connection.flush()
}

fn get_request(connection: &mut TcpStream) -> io::Result<()> {
    let mut read_num = 0;
    let mut request_buffer = [0u8; 1024];

    loop {
        let num_bytes = connection.read(&mut request_buffer[read_num..])?;

        // the client disconnected
        if num_bytes == 0 {
            println!("client disconnected unexpectedly ");
            return Ok(());
        }

        // next start index
        read_num += num_bytes;

        // check if arrives the end by "\r\n\r\n"
        if let Some(b"\r\n\r\n") = request_buffer.get((read_num - 4)..read_num) {
            break;
        }
    }
    let request = String::from_utf8_lossy(&request_buffer);
    println!("request: {request}");
    Ok(())
}


fn send_response(connection: &mut TcpStream) -> io::Result<()> {
    let response = concat!(
    "HTTP/1.1 200 OK\r\n",
    "Content-Length: 12\n",
    "Connection: close\r\n\r\n",
    "Hello world"
    );

    let mut written = 0;
    loop {
        let num_bytes = connection.write(&response[written..].as_bytes())?;

        if num_bytes == 0 {
            println!("client disconnected unexpectedly");
            return Ok(());
        }

        written += num_bytes;

        // All has been written
        if written == response.len() {
            println!("End the response");
            break;
        }
    }
    Ok(())
}
