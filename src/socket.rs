use std::{
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use crate::consts::DEF_LPR_PORT;

pub fn create_lpr_sock(host: impl ToString, timeout: Duration) -> std::io::Result<TcpStream> {
    let deadline = std::time::Instant::now() + timeout;

    let (tx, rx) = std::sync::mpsc::channel();
    let host = host.to_string();

    std::thread::spawn(move || {
        let result = (host, DEF_LPR_PORT).to_socket_addrs();
        let _ = tx.send(result);
    });

    let addrs = match rx.recv_timeout(timeout) {
        Ok(Ok(addrs)) => addrs,
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "DNS resolution timed out",
            ));
        }
    };

    let mut last_error = None;
    for addr in addrs {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match TcpStream::connect_timeout(&addr, remaining) {
            Ok(socket) => return Ok(socket),
            Err(error) => last_error = Some(error),
        }
    }

    return Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::AddrNotAvailable,
            "no socket addresses resolved",
        )
    }));
}
