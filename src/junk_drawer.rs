use failure::Error;
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::*;

use failure::ResultExt;

// We do this shenanigans to (hopefully) avoid a race condition where
// two threads test that a port is "free" one after the other, but before
// either is able to start it's driver.
static PORT: AtomicUsize = ATOMIC_USIZE_INIT;

pub fn unused_port_no() -> Result<u16, Error> {
    let start_port = 4444u16;
    loop {
        let off = PORT.fetch_add(1, Ordering::SeqCst);
        let port = start_port
            .checked_add(off as u16)
            .ok_or_else(|| failure::err_msg("Allocated more ports than we have namespace for?"))?;
        let a = SocketAddr::from(([127, 0, 0, 1], port));
        debug!("Trying to bind to address: {:?}", a);
        if let Some(l) = TcpListener::bind(a)
            .map(Some)
            .or_else(|e| {
                if e.kind() == std::io::ErrorKind::AddrInUse {
                    info!("Retrying");
                    Ok(None)
                } else {
                    warn!("Error binding to {:?}; kind:{:?}; {:?}", a, e.kind(), e);
                    Err(e)
                }
            }).context("Binding to ephemeral port")?
        {
            let addr = l.local_addr().context("Listener local port")?;
            info!("Available: {}", addr);
            return Ok(addr.port());
        }
    }
}
