macro_rules! debug_log {
    ($l:expr, $($arg:tt)*) => {
        let _ = (if *crate::DEBUG {
            let format = format!(
                "[{}] {}",
                match $l {
                    LogLevel::Error => "ERR",
                    LogLevel::Warn => "WARNING",
                    LogLevel::Ok => "OK"
                },
                format_args!($($arg)*)
            );

            match $l {
                LogLevel::Ok => println!("{}", format),
                _ => eprintln!("{}", format)
            }
        });
    };
}

macro_rules! sock {
    ($x:ident, $h:ident, $p:ident, $t:expr) => {
        let $x = (|| -> std::io::Result<TcpStream> {
            let timeout = $t;
            let deadline = std::time::Instant::now() + timeout;

            let host = $h.to_string();
            let port = $p;
            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let result = (host.as_str(), port).to_socket_addrs();
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
            Err(last_error.unwrap_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::AddrNotAvailable,
                    "no socket addresses resolved",
                )
            }))
        })();
    };
}
