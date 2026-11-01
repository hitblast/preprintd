#[cfg(target_os = "linux")]
use zbus::{OwnedFd, Proxy, connection::Connection};

#[cfg(target_os = "linux")]
use crate::ALIAS;

#[cfg(target_os = "linux")]
async fn acquire_backend() -> zbus::Result<OwnedFd> {
    let proxy = Proxy::new(
        &Connection::system().await?,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await?;

    let fd: OwnedFd = proxy
        .call(
            "Inhibit",
            &(
                "sleep",
                ALIAS.as_str(),
                "Required for sustained streaming operation.",
                "block",
            ),
        )
        .await?;

    Ok(fd)
}

#[cfg(target_os = "linux")]
pub fn acquire_sleep_inhibitor() -> zbus::Result<OwnedFd> {
    zbus::block_on(acquire_backend())
}
