use std::net::TcpListener;
use std::io;
use std::io::{Read, Write};


enum ConnectionState {
    Read {
        read_point: usize,
        request_buf: [u8; 1024],
    },
    Write {
        written_point: usize,
        response: &'static [u8],
    },
    Flush,
}

fn main() -> io::Result<()> {
    // Get a nonblocking TcpListener
    let listener = TcpListener::bind("localhost:30000")?;
    listener.set_nonblocking(true)?;

    // Store every connection info until it has been removed
    let mut connection_list = vec![];
    loop {
        match listener.accept() {
            Ok((connection, _)) => {
                connection.set_nonblocking(true)?;

                // initial the state of the new connection
                let state = ConnectionState::Read {
                    request_buf: [0; 1024],
                    read_point: 0,
                };

                // Place it in the end of tasks' queue
                connection_list.push((connection, state));
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // While there no resource to accept connection , abandon this connection !
                continue;
            }
            Err(e) => panic!("{e}")
        };

        // store the task finished index in connection_list
        let mut completed_indices = vec![];

        // Those previously  connections still need to be move forward
        // as they may be skipped  due to not acquiring resources during read or write operations.
        'next: for (i, (connection, state)) in connection_list.iter_mut().enumerate() {
            if let ConnectionState::Read { request_buf, read_point } = state {
                loop {
                    match connection.read(&mut request_buf[*read_point..]) {
                        Ok(0) => {
                            // While connection is closed by client, add the connection index to abandon list
                            println!("Disconnected by client! ");
                            completed_indices.push(i);
                            continue 'next;
                        }
                        Ok(n) => { *read_point += n }
                        // The resource not ready yet , move on to the next connection
                        // Don't worry the request has been read into the buffer , ConnectionState::Read owns the buffer content and offset
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue 'next,
                        Err(e) => panic!("{e}")
                    }
                    // break from the loop when request has completed 'read' into buffer
                    if request_buf.get((*read_point - 4)..*read_point) == Some(b"\r\n\r\n") { break; }
                }
                // log
                let request = String::from_utf8_lossy(&request_buf[..*read_point]);
                println!("{request}");

                // and move on to write response
                let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Length: 12\n",
                "Connection: close\r\n\r\n",
                "Hello world!"
                ).as_bytes();
                *state = ConnectionState::Write { response, written_point: 0 };
            }
            // Now we can response this connection based on  the request
            if let ConnectionState::Write { mut response, written_point } = state {
                loop {
                    match connection.write(&mut response) {
                        Ok(0) => {
                            println!("Disconnected  by client");
                            completed_indices.push(i);
                            continue 'next;
                        }
                        Ok(n) => {
                            *written_point += n;
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            continue 'next;
                        }
                        Err(e) => {
                            panic!("{e}");
                        }
                    }

                    if response.len() == *written_point {
                        break;
                    }
                }

                *state = ConnectionState::Flush;
            }
            if let ConnectionState::Flush = state {
                match connection.flush() {
                    Ok(_) => {
                        completed_indices.push(i);
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        // not ready yet, move on to the next connection
                        continue 'next;
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        }

        // iterate in reverse order to preserve the indices
        for i in completed_indices.into_iter().rev() {
            connection_list.remove(i);
        }
    }
}

