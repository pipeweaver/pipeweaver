use std::ops::DerefMut;

use actix::{
    Actor, ActorContext, AsyncContext, ContextFutureSpawner, Handler, Message, StreamHandler,
    WrapFuture,
};
use actix_cors::Cors;
use actix_web::dev::ServerHandle;
use actix_web::http::header::ContentType;
use actix_web::middleware::Condition;
use actix_web::web::Data;
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use actix_web_actors::ws::{CloseCode, CloseReason};
use anyhow::{anyhow, Result};
use include_dir::{include_dir, Dir};
use json_patch::Patch;
use log::{debug, error, info, warn};
use mime_guess::MimeGuess;
use serde_json::Value;
use tokio::sync::broadcast::Sender as BroadcastSender;
use tokio::sync::oneshot::Sender;
use tokio::sync::Mutex;

use crate::handler::packet::{handle_packet, Messenger};
use crate::APP_NAME;
use ipc::commands::{
    DaemonRequest, DaemonResponse, DaemonStatus, HttpSettings, WebsocketRequest, WebsocketResponse,
};

const WEB_CONTENT: Dir = include_dir!("./daemon/web-content/");

#[derive(Debug, Clone)]
pub struct PatchEvent {
    pub data: Patch,
}

struct Websocket {
    usb_tx: Messenger,
    broadcast_tx: BroadcastSender<PatchEvent>,
}

impl Actor for Websocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();
        let mut broadcast_rx = self.broadcast_tx.subscribe();

        // Create a future that simply monitors the global broadcast bus, and pushes any changes
        // out to the WebSocket.
        let future = Box::pin(async move {
            loop {
                if let Ok(event) = broadcast_rx.recv().await {
                    // We've received a message, attempt to trigger the WsMessage Handle..
                    if let Err(error) = address.clone().try_send(WsResponse(WebsocketResponse {
                        id: u64::MAX,
                        data: DaemonResponse::Patch(event.data),
                    })) {
                        error!(
                            "Error Occurred when sending message to websocket: {:?}",
                            error
                        );
                        warn!("Aborting Websocket pushes for this client.");
                        break;
                    }
                }
            }
        });

        let future = future.into_actor(self);
        ctx.spawn(future);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct WsResponse(WebsocketResponse);

impl Handler<WsResponse> for Websocket {
    type Result = ();

    fn handle(&mut self, msg: WsResponse, ctx: &mut Self::Context) -> Self::Result {
        if let Ok(result) = serde_json::to_string(&msg.0) {
            ctx.text(result);
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Websocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                match serde_json::from_slice::<WebsocketRequest>(text.as_ref()) {
                    Ok(request) => {
                        let recipient = ctx.address().recipient();
                        let usb_tx = self.usb_tx.clone();
                        let future = async move {
                            let request_id = request.id;
                            let result = handle_packet(request.data, usb_tx).await;
                            match result {
                                Ok(resp) => match resp {
                                    DaemonResponse::Ok => {
                                        recipient.do_send(WsResponse(WebsocketResponse {
                                            id: request_id,
                                            data: DaemonResponse::Ok,
                                        }));
                                    }
                                    DaemonResponse::Err(error) => {
                                        recipient.do_send(WsResponse(WebsocketResponse {
                                            id: request_id,
                                            data: DaemonResponse::Err(error),
                                        }));
                                    }
                                    DaemonResponse::Status(status) => {
                                        recipient.do_send(WsResponse(WebsocketResponse {
                                            id: request_id,
                                            data: DaemonResponse::Status(status),
                                        }));
                                    }
                                    DaemonResponse::Pipewire(result) => {
                                        recipient.do_send(WsResponse(WebsocketResponse {
                                            id: request_id,
                                            data: DaemonResponse::Pipewire(result),
                                        }));
                                    }
                                    _ => {
                                        panic!("Unexpected Response!");
                                    }
                                },
                                Err(error) => {
                                    recipient.do_send(WsResponse(WebsocketResponse {
                                        id: request_id,
                                        data: DaemonResponse::Err(error.to_string()),
                                    }));
                                }
                            }
                        };
                        future.into_actor(self).spawn(ctx);
                    }
                    Err(error) => {
                        // Ok, we weren't able to deserialise the request into a proper object, we
                        // now need to confirm whether it was at least valid JSON with a request id
                        warn!("Error deserialising Request to Object: {}", error);
                        warn!("Original Request: {}", text);

                        debug!("Attempting Low Level request Id Extraction..");
                        let request: serde_json::Result<Value> =
                            serde_json::from_str(text.as_ref());
                        match request {
                            Ok(value) => {
                                if let Some(request_id) = value["id"].as_u64() {
                                    let recipient = ctx.address().recipient();
                                    recipient.do_send(WsResponse(WebsocketResponse {
                                        id: request_id,
                                        data: DaemonResponse::Err(error.to_string()),
                                    }));
                                } else {
                                    warn!("id missing, Cannot continue. Closing connection");
                                    let error = CloseReason {
                                        code: CloseCode::Invalid,
                                        description: Some(String::from(
                                            "Missing or invalid Request ID",
                                        )),
                                    };
                                    ctx.close(Some(error));
                                    ctx.stop();
                                }
                            }
                            Err(error) => {
                                warn!("JSON structure is invalid, closing connection.");
                                let error = CloseReason {
                                    code: CloseCode::Invalid,
                                    description: Some(error.to_string()),
                                };
                                ctx.close(Some(error));
                                ctx.stop();
                            }
                        }
                    }
                }
            }
            Ok(ws::Message::Binary(_bin)) => {
                ctx.close(Some(CloseCode::Unsupported.into()));
                ctx.stop();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => (),
        }
    }
}

struct AppData {
    messenger: Messenger,
    broadcast_tx: BroadcastSender<PatchEvent>,
}

pub async fn spawn_http_server(
    messenger: Messenger,
    handle_tx: Sender<ServerHandle>,
    broadcast_tx: tokio::sync::broadcast::Sender<PatchEvent>,
    settings: HttpSettings,
) {
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
            .app_data(Data::new(Mutex::new(AppData {
                broadcast_tx: broadcast_tx.clone(),
                messenger: messenger.clone(),
            })))
            .service(execute_command)
            .service(get_devices)
            .service(websocket)
            .default_service(web::to(default))
    }).bind((settings.bind_address.clone(), settings.port));

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
        info!("HTTP Server Stopped.");
    } else {
        warn!("HTTP Server Stopped with Error");
    }
}

#[get("/api/websocket")]
async fn websocket(
    usb_mutex: Data<Mutex<AppData>>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    let data = usb_mutex.lock().await;

    ws::start(
        Websocket {
            usb_tx: data.messenger.clone(),
            broadcast_tx: data.broadcast_tx.clone(),
        },
        &req,
        stream,
    )
}

// So, fun note, according to the actix manual, web::Json uses serde_json to deserialise, good
// news everybody! So do we.. :)
#[post("/api/command")]
async fn execute_command(
    request: web::Json<DaemonRequest>,
    app_data: Data<Mutex<AppData>>,
) -> HttpResponse {
    let mut guard = app_data.lock().await;
    let sender = guard.deref_mut();

    // Errors propagate weirdly in the javascript world, so send all as OK, and handle there.
    match handle_packet(request.0, sender.messenger.clone()).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => HttpResponse::Ok().json(DaemonResponse::Err(error.to_string())),
    }
}

#[get("/api/get-devices")]
async fn get_devices(app_data: Data<Mutex<AppData>>) -> HttpResponse {
    if let Ok(response) = get_status(app_data).await {
        return HttpResponse::Ok().json(&response);
    }
    HttpResponse::InternalServerError().finish()
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

async fn get_status(app_data: Data<Mutex<AppData>>) -> Result<DaemonStatus> {
    let mut guard = app_data.lock().await;
    let sender = guard.deref_mut();

    let request = DaemonRequest::GetStatus;

    let result = handle_packet(request, sender.messenger.clone()).await?;
    match result {
        DaemonResponse::Status(status) => Ok(status),
        _ => Err(anyhow!("Unexpected Daemon Status Result: {:?}", result)),
    }
}
