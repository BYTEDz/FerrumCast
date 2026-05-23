use crate::config::{Capabilities, StreamConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(target_os = "linux")]
use tokio::net::UnixListener;
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InboundMessage {
    SetRemoteSdp {
        sdp: String,
        sdp_type: String,
    },
    AddIceCandidate {
        candidate: String,
        #[serde(rename = "sdpMid")]
        mid: Option<String>,
        #[serde(rename = "sdpMLineIndex")]
        mline_index: Option<u32>,
    },
    RequestOffer,
    StopStream,
    ConfigureStream(StreamConfig),
    GetCapabilities,
    RestartPipeline(StreamConfig),
    ForceKeyframe,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutboundMessage {
    LocalSdpGenerated {
        sdp: String,
        sdp_type: String,
    },
    LocalIceCandidate {
        candidate: String,
        #[serde(rename = "sdpMid")]
        mid: Option<String>,
        #[serde(rename = "sdpMLineIndex")]
        mline_index: Option<u32>,
    },
    StreamError {
        message: String,
    },
    CapabilitiesResponse(Capabilities),
    ConfigAck {
        active_encoder: String,
    },
    PortalTokenGenerated {
        token: String,
    },
    WaitingForPortalApproval,
}

pub struct IpcServer {
    path: String,
}

impl IpcServer {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    pub async fn run<F, Fut>(
        self: Arc<Self>,
        handler: F,
        global_tx: tokio::sync::broadcast::Sender<OutboundMessage>,
    ) -> Result<()>
    where
        F: Fn(InboundMessage) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let handler = Arc::new(handler);

        #[cfg(target_os = "linux")]
        {
            if Path::new(&self.path).exists() {
                let _ = std::fs::remove_file(&self.path);
            }

            let listener = UnixListener::bind(&self.path)?;

            // Restrict socket permissions to the owner to prevent unauthorized local privilege escalation.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
            }

            info!("IPC listening on {}", self.path);

            loop {
                let (socket, _) = match listener.accept().await {
                    Ok(s) => s,
                    Err(e) => {
                        error!("accept error: {}", e);
                        continue;
                    }
                };
                self.spawn_client_task(socket, handler.clone(), global_tx.subscribe())
                    .await;
            }
        }

        #[cfg(target_os = "windows")]
        {
            use tokio::net::windows::named_pipe::ServerOptions;
            info!("IPC listening on named pipe: {}", self.path);

            // Enforce the first-pipe-instance constraint on Windows to mitigate named pipe hijacking or spoofing.
            let mut server = ServerOptions::new()
                .first_pipe_instance(true)
                .create(&self.path)?;

            loop {
                if let Err(e) = server.connect().await {
                    error!("pipe connect error: {}", e);
                    // Re-instantiate the pipe instance on failure to ensure continuous service availability.
                    server = ServerOptions::new().create(&self.path)?;
                    continue;
                }

                let connected_client = server;
                // Pre-allocate the next pipe instance before spawning the active client task to minimize
                // the connection-window gap and prevent dropped concurrent connection requests.
                server = ServerOptions::new().create(&self.path)?;

                self.spawn_client_task(connected_client, handler.clone(), global_tx.subscribe())
                    .await;
            }
        }
    }

    async fn spawn_client_task<S, F, Fut>(
        &self,
        stream: S,
        handler: Arc<F>,
        mut global_rx: tokio::sync::broadcast::Receiver<OutboundMessage>,
    ) where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
        F: Fn(InboundMessage) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        info!("client connected");
        tokio::spawn(async move {
            let (reader, mut writer) = tokio::io::split(stream);
            let mut buf_reader = tokio::io::BufReader::new(reader);

            let read_task = async {
                let mut line = String::new();
                loop {
                    line.clear();
                    // Bound the maximum line length to 1MB to prevent memory exhaustion (OOM) or DoS from malicious inputs.
                    let mut handle = (&mut buf_reader).take(1024 * 1024);
                    match tokio::io::AsyncBufReadExt::read_line(&mut handle, &mut line).await {
                        Ok(0) => break,
                        Ok(_) => {
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            if let Ok(msg) = serde_json::from_str::<InboundMessage>(trimmed) {
                                handler(msg).await;
                            }
                        }
                        Err(_) => break,
                    }
                }
            };

            let write_task = async {
                while let Ok(msg) = global_rx.recv().await {
                    if let Ok(mut data) = serde_json::to_vec(&msg) {
                        data.push(b'\n');
                        if writer.write_all(&data).await.is_err() {
                            break;
                        }
                    }
                }
            };

            tokio::select! {
                _ = read_task => {},
                _ = write_task => {},
            }
            info!("client disconnected");
        });
    }
}