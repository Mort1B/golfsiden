//! Test-only PostgreSQL response gate. SQL is forwarded unchanged; only the
//! ordinary BEGIN/SAVEPOINT acknowledgement is held, without timer-based races.
use sqlx::{
    PgPool,
    postgres::{PgPoolOptions, PgSslMode},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Notify,
    task::{JoinHandle, JoinSet},
};

#[derive(Default)]
pub struct Gate {
    armed: AtomicBool,
    pub reached: Notify,
    pub release: Notify,
    commands: std::sync::Mutex<Vec<String>>,
}
impl Gate {
    pub fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }
    pub fn commands(&self) -> Vec<String> {
        self.commands.lock().unwrap().clone()
    }
}
pub struct Proxy {
    pub gate: Arc<Gate>,
    task: JoinHandle<()>,
}
impl Drop for Proxy {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Proxy {
    pub async fn pool(source: &PgPool) -> (Self, PgPool) {
        let options = source.connect_options();
        let upstream = (options.get_host().to_owned(), options.get_port());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let gate = Arc::new(Gate::default());
        let shared = gate.clone();
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (client, _) = accepted.unwrap();
                        client.set_nodelay(true).unwrap();
                        let server = TcpStream::connect((upstream.0.as_str(), upstream.1)).await.unwrap();
                        server.set_nodelay(true).unwrap();
                        let gate = shared.clone();
                        connections.spawn(async move {
                            let (mut cr, mut cw) = client.into_split();
                            let (mut sr, mut sw) = server.into_split();
                            let to_server = tokio::io::copy(&mut cr, &mut sw);
                            let to_client = async {
                                loop {
                                    let tag = sr.read_u8().await?;
                                    let len = sr.read_u32().await?;
                                    assert!((4..=16_777_216).contains(&len));
                                    let mut body = vec![0; (len - 4) as usize];
                                    sr.read_exact(&mut body).await?;
                                    if tag == b'C' {
                                        let command = String::from_utf8_lossy(&body).trim_end_matches('\0').to_owned();
                                        gate.commands.lock().unwrap().push(command.clone());
                                        if (command == "BEGIN" || command == "SAVEPOINT") && gate.armed.swap(false, Ordering::SeqCst) {
                                            gate.reached.notify_one();
                                            gate.release.notified().await;
                                        }
                                    }
                                    cw.write_u8(tag).await?;
                                    cw.write_u32(len).await?;
                                    cw.write_all(&body).await?;
                                }
                                #[allow(unreachable_code)]
                                Ok::<(), std::io::Error>(())
                            };
                            tokio::select! { _ = to_server => {}, _ = to_client => {} }
                        });
                    }
                    _ = connections.join_next(), if !connections.is_empty() => {}
                }
            }
        });
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect_with(
                (*options)
                    .clone()
                    .host("127.0.0.1")
                    .port(port)
                    .ssl_mode(PgSslMode::Disable),
            )
            .await
            .unwrap();
        (Self { gate, task }, pool)
    }
}
