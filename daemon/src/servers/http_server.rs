use crate::APP_NAME;
use crate::handler::packet::{Messenger, handle_packet};
use actix_cors::Cors;
use actix_web::dev::ServerHandle;
use actix_web::http::header::ContentType;
use actix_web::middleware::Condition;
use actix_web::web::Data;
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, get, post, web};
use actix_ws::{AggregatedMessage, CloseCode, CloseReason, Session};
use anyhow::{Result, anyhow};
use futures_lite::StreamExt;
use include_dir::{Dir, include_dir};
use json_patch::Patch;
use log::{debug, error, info, warn};
use mime_guess::MimeGuess;
use pipeweaver_ipc::commands::DaemonCommand::SetMetering;
use pipeweaver_ipc::commands::{
    DaemonRequest, DaemonResponse, DaemonStatus, HttpSettings, WebsocketRequest, WebsocketResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;
use tokio::sync::broadcast::Sender as BroadcastSender;
use tokio::sync::oneshot::Sender;
use ulid::Ulid;

const WEB_CONTENT: Dir = include_dir!("./daemon/web-content/");
type ClientCounter = Arc<AtomicUsize>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterEvent {
    pub(crate) id: Ulid,
    pub(crate) percent: u8,
}

#[derive(Debug, Clone)]
pub struct PatchEvent {
    pub data: Patch,
}

#[derive(Serialize)]
struct WsResponse(WebsocketResponse);

struct AppData {
    messenger: Messenger,
    broadcast_tx: BroadcastSender<PatchEvent>,
    meter_tx: BroadcastSender<MeterEvent>,
    client_counter: ClientCounter,
}

pub async fn spawn_http_server(
    messenger: Messenger,
    handle_tx: Sender<ServerHandle>,
    broadcast_tx: tokio::sync::broadcast::Sender<PatchEvent>,
    meter_tx: tokio::sync::broadcast::Sender<MeterEvent>,
    settings: HttpSettings,
) {
    let client_counter = Arc::new(AtomicUsize::new(0));
    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _req_head| {
                origin.as_bytes().starts_with(b"http://127.0.0.1")
                    || origin.as_bytes().starts_with(b"http://localhost")
            })
            .allow_any_method()
            .allow_any_header()
            .max_age(300);
        App::new()
            .wrap(Condition::new(settings.cors_enabled, cors))
            .app_data(Data::new(RwLock::new(AppData {
                messenger: messenger.clone(),
                broadcast_tx: broadcast_tx.clone(),
                meter_tx: meter_tx.clone(),
                client_counter: client_counter.clone(),
            })))
            .service(execute_command)
            .service(get_devices)
            .service(websocket)
            .service(websocket_meter)
            .default_service(web::to(default))
    })
    .bind((settings.bind_address.clone(), settings.port));

    if let Err(e) = server {
        warn!("Error Running HTTP Server: {:#?}", e);
        return;
    }

    let server = server.unwrap().run();
    info!(
        "Started {} configuration interface at http://{}:{}/",
        APP_NAME,
        settings.bind_address.as_str(),
        settings.port,
    );

    let _ = handle_tx.send(server.handle());

    if server.await.is_ok() {
        info!("[HTTP] Stopped");
    } else {
        warn!("[HTTP] Stopped with Error");
    }
}

#[get("/api/websocket")]
async fn websocket(
    app_data: Data<RwLock<AppData>>,
    req: HttpRequest,
    body: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    let (response, mut session, msg_stream) = actix_ws::handle(&req, body)?;

    let data = app_data.read().await;
    let usb_tx = data.messenger.clone();
    let mut broadcast_rx = data.broadcast_tx.subscribe();

    actix_web::rt::spawn(async move {
        let mut msg_stream = msg_stream.aggregate_continuations();
        let close_reason = loop {
            tokio::select! {
                Ok(patch) = broadcast_rx.recv() => {
                    let message = WsResponse(WebsocketResponse {
                        id: u64::MAX,
                        data: DaemonResponse::Patch(patch.data),
                    });
                    if let Err(e) = send_message(&message, &mut session).await {
                        break e;
                    }
                }
                Some(Ok(msg)) = msg_stream.next() => {
                    match msg {
                        AggregatedMessage::Ping(msg) => {
                            if let Err(e) = session.pong(&msg).await {
                                error!("Failed to send Pong: {}", e);
                                break Some(CloseReason {
                                    code: CloseCode::Error,
                                    description: Some(format!("Failed to Send Pong: {}", e)),
                                });
                            };
                        }
                        AggregatedMessage::Text(msg) => {
                            match serde_json::from_slice::<WebsocketRequest>(msg.as_ref()) {
                                Ok(request) => {
                                    let request_id = request.id;
                                    let result = handle_packet(request.data, &usb_tx).await;
                                    let response = match result {
                                        Ok(resp) => {
                                            match resp {
                                                DaemonResponse::Ok => {
                                                    WsResponse(WebsocketResponse {
                                                        id: request_id,
                                                        data: DaemonResponse::Ok,
                                                    })
                                                }
                                                DaemonResponse::Err(error) => {
                                                    WsResponse(WebsocketResponse {
                                                        id: request_id,
                                                        data: DaemonResponse::Err(error),
                                                    })
                                                }
                                                DaemonResponse::Status(status) => {
                                                    WsResponse(WebsocketResponse {
                                                        id: request_id,
                                                        data: DaemonResponse::Status(status),
                                                    })
                                                }
                                                DaemonResponse::Pipewire(result) => {
                                                    WsResponse(WebsocketResponse {
                                                        id: request_id,
                                                        data: DaemonResponse::Pipewire(result),
                                                    })
                                                }
                                                _ => {
                                                    // This should never fucking happen
                                                    break Some(CloseReason {
                                                        code: CloseCode::Abnormal,
                                                        description: Some("Unexpected Message Type".to_string()),
                                                    });
                                                }
                                            }
                                        },
                                        Err(error) => {
                                            WsResponse(WebsocketResponse {
                                                id: request_id,
                                                data: DaemonResponse::Err(error.to_string()),
                                            })
                                        }
                                    };
                                    if let Err(e) = send_message(&response, &mut session).await {
                                        break e;
                                    }
                                }
                                Err(error) => {
                                    // Ok, we weren't able to deserialise the request into a proper object, we
                                    // now need to confirm whether it was at least valid JSON with a request id
                                    warn!("Error Deserialising Request to Object: {}", error);
                                    warn!("Original Request: {}", msg);

                                    debug!("Attempting Low Level request Id Extraction..");
                                    let request: serde_json::Result<Value> = serde_json::from_str(msg.as_ref());
                                    match request {
                                        Ok(value) => {
                                            if let Some(request_id) = value["id"].as_u64() {
                                                let response = WsResponse(WebsocketResponse {
                                                    id: request_id,
                                                    data: DaemonResponse::Err(error.to_string()),
                                                });
                                                if let Err(e) = send_message(&response, &mut session).await {
                                                    break e;
                                                }
                                            } else {
                                                warn!("id missing, Cannot continue. Closing connection");
                                                let error = CloseReason {
                                                    code: CloseCode::Invalid,
                                                    description: Some(String::from(
                                                        "Missing or invalid Request ID",
                                                    )),
                                                };
                                                break Some(error);
                                            }
                                        }
                                        Err(error) => {
                                            warn!("JSON structure is invalid, closing connection.");
                                            let error = CloseReason {
                                                code: CloseCode::Invalid,
                                                description: Some(error.to_string()),
                                            };
                                            break Some(error);
                                        }
                                    }
                                }
                            }
                        }
                        AggregatedMessage::Binary(_) => {
                            error!("Received Binary Message, aborting!");
                            break Some(CloseReason {
                                code: CloseCode::Unsupported,
                                description: Some("Binary is not Supported".to_string()),
                            });
                        }
                        AggregatedMessage::Pong(_) => {}
                        AggregatedMessage::Close(reason) => {
                            break reason;
                        }
                    }
                }
                else => {
                    break None;
                }
            }
        };

        let _ = session.close(close_reason).await;
    });

    Ok(response)
}

#[get("/api/websocket/meter")]
async fn websocket_meter(
    app_data: Data<RwLock<AppData>>,
    req: HttpRequest,
    body: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    let (response, mut session, msg_stream) = actix_ws::handle(&req, body)?;
    let data = app_data.read().await;
    let messenger = data.messenger.clone();
    let client_counter = data.client_counter.clone();
    let mut meter_rx = data.meter_tx.subscribe();

    actix_web::rt::spawn(async move {
        // Is this the first client?
        if client_counter.fetch_add(1, Ordering::SeqCst) == 0 {
            debug!("First Client Connected, starting metering...");
            let request = DaemonRequest::Daemon(SetMetering(true));
            let _ = handle_packet(request, &messenger).await;
        }

        let mut msg_stream = msg_stream.aggregate_continuations();
        let close_reason = loop {
            tokio::select! {
                Ok(event) = meter_rx.recv() => {
                    if let Err(e) = send_message(&event, &mut session).await {
                        break e;
                    }
                }
                Some(Ok(msg)) = msg_stream.next() => {
                    match msg {
                        AggregatedMessage::Ping(msg) => {
                            if let Err(e) = session.pong(&msg).await {
                                error!("Failed to send Pong: {}", e);
                                break Some(CloseReason {
                                    code: CloseCode::Error,
                                    description: Some(format!("Failed to Send Pong: {}", e)),
                                });
                            };
                        }
                        AggregatedMessage::Text(_) => {
                            error!("Received Text Message, aborting!");
                            break Some(CloseReason {
                                code: CloseCode::Unsupported,
                                description: Some("This socket expects no input".to_string()),
                            });
                        }
                        AggregatedMessage::Binary(_) => {
                            error!("Received Binary Message, aborting!");
                            break Some(CloseReason {
                                code: CloseCode::Unsupported,
                                description: Some("Binary is not Supported".to_string()),
                            });
                        }
                        AggregatedMessage::Pong(_) => {}
                        AggregatedMessage::Close(reason) => {
                            break reason;
                        }
                    }
                }
                else => {
                    break None;
                }
            }
        };

        debug!("Session Disconnected: {:?}", close_reason);
        let _ = session.close(close_reason).await;

        // If we're metering, and this is the last client, stop metering
        if client_counter.fetch_sub(1, Ordering::SeqCst) == 1 {
            // Last client disconnected
            debug!("Last Client disconnected, stopping metering");
            let request = DaemonRequest::Daemon(SetMetering(false));
            let _ = handle_packet(request, &messenger).await;
        }
    });
    Ok(response)
}

// So, fun note, according to the actix manual, web::Json uses serde_json to deserialise, good
// news everybody! So do we.. :)
#[post("/api/command")]
async fn execute_command(
    request: web::Json<DaemonRequest>,
    app_data: Data<RwLock<AppData>>,
) -> HttpResponse {
    let data = app_data.read().await;

    // Errors propagate weirdly in the javascript world, so send all as OK, and handle there.
    match handle_packet(request.0, &data.messenger).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => HttpResponse::Ok().json(DaemonResponse::Err(error.to_string())),
    }
}

#[get("/api/get-devices")]
async fn get_devices(app_data: Data<RwLock<AppData>>) -> HttpResponse {
    if let Ok(response) = get_status(app_data).await {
        return HttpResponse::Ok().json(&response);
    }
    HttpResponse::InternalServerError().finish()
}

/// Serialises a serialisable into a JSON mess, and send to websocket
async fn send_message<T>(value: &T, session: &mut Session) -> Result<(), Option<CloseReason>>
where
    T: Serialize,
{
    match serde_json::to_string(value) {
        Ok(text) => {
            if let Err(e) = session.text(text).await {
                error!("Failed to send message: {}", e);
                return Err(Some(CloseReason {
                    code: CloseCode::Error,
                    description: Some(e.to_string()),
                }));
            }
        }
        Err(e) => {
            error!("Failed to serialize message: {}", e);
            return Err(Some(CloseReason {
                code: CloseCode::Error,
                description: Some(format!("Serialization Error: {}", e)),
            }));
        }
    }
    Ok(())
}

async fn default(req: HttpRequest) -> HttpResponse {
    let path = if req.path() == "/" || req.path() == "" {
        "/index.html"
    } else {
        req.path()
    };
    let path_part = &path[1..path.len()];
    let file = WEB_CONTENT.get_file(path_part);
    if let Some(file) = file {
        let mime_type = MimeGuess::from_path(path).first_or_octet_stream();
        let mut builder = HttpResponse::Ok();
        builder.insert_header(ContentType(mime_type));
        builder.body(file.contents())
    } else {
        HttpResponse::NotFound().finish()
    }
}

async fn get_status(app_data: Data<RwLock<AppData>>) -> Result<DaemonStatus> {
    let data = app_data.read().await;
    let request = DaemonRequest::GetStatus;

    let result = handle_packet(request, &data.messenger).await?;
    match result {
        DaemonResponse::Status(status) => Ok(status),
        _ => Err(anyhow!("Unexpected Daemon Status Result: {:?}", result)),
    }
}
