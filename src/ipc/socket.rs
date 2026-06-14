use crate::clipboard::history::ClipItem;
use crate::store::Store;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

/// Default path to the daemon's IPC socket.
pub fn socket_path(store: &Store) -> std::path::PathBuf {
    store.data_dir().join("daemon.sock")
}

/// Start the IPC server in a background thread.  Returns a receiver
/// that yields `ClipItem` paste requests sent by popup processes.
///
/// The server listens on a Unix domain socket instead of polling the
/// filesystem, making daemon↔popup communication truly event-driven.
pub fn start_server(socket: &Path) -> std::io::Result<mpsc::Receiver<ClipItem>> {
    // Remove stale socket file from a previous run
    let _ = std::fs::remove_file(socket);

    let listener = UnixListener::bind(socket)?;
    listener.set_nonblocking(true)?;

    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let mut buf = Vec::with_capacity(4096);
                    if stream.read_to_end(&mut buf).is_ok() {
                        if let Ok(item) = serde_json::from_slice::<ClipItem>(&buf) {
                            if tx.send(item).is_err() {
                                break; // receiver dropped, daemon shutting down
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No pending connections — sleep briefly before retrying
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    });

    Ok(rx)
}

/// Send a paste request to the daemon via the IPC socket.
/// Returns `Ok(())` if the request was successfully sent,
/// or an error if the daemon socket is not available.
pub fn send_paste_request(store: &Store, item: &ClipItem) -> std::io::Result<()> {
    let path = socket_path(store);
    let mut stream = UnixStream::connect(&path)?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let json =
        serde_json::to_vec(item).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    stream.write_all(&json)?;
    stream.shutdown(std::net::Shutdown::Write)?;
    Ok(())
}
