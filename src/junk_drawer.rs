use failure::Error;
use std::net::{SocketAddr, TcpListener};

use failure::ResultExt;


pub fn unused_port_no() -> Result<u16, Error> {
    let start_port = 4444u16;
    for port in start_port.. {
        let a = SocketAddr::from(([127, 0, 0, 1], port));
        debug!("Trying to bind to address: {:?}", a);
        if let Some(l) = TcpListener::bind(a)
            .map(Some)
            .or_else(|e| if e.kind() == std::io::ErrorKind::AddrInUse {
                info!("Retrying");
                Ok(None)
            } else {
                warn!("Error binding to {:?}; kind:{:?}; {:?}", a, e.kind(), e);
                Err(e)
            })
            .context("Binding to ephemeral port")?
        {
            let addr = l.local_addr().context("Listener local port")?;
            info!("Available: {}", addr);
            return Ok(addr.port());
        }
    }
    bail!("Could not find un-used port from {}..", start_port)
}
