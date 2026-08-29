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
