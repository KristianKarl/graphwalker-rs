use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{State, WebSocketUpgrade};
use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use clap::Parser;
use graphwalker_restful::session::SessionManager;
use include_dir::{include_dir, Dir};
use tower_http::services::ServeDir;

static EMBEDDED_STATIC: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

#[derive(Parser)]
#[command(
    name = "graphwalker-studio",
    about = "GraphWalker Studio – visual model editor"
)]
struct Args {
    /// HTTP port for the web UI
    #[arg(short = 'b', long = "browser-port", default_value_t = 9090)]
    browser_port: u16,

    /// WebSocket port for execution engine
    #[arg(short = 'w', long = "websocket-port", default_value_t = 9999)]
    websocket_port: u16,

    /// Directory containing static web files
    #[arg(long = "static-dir", default_value = "static")]
    static_dir: PathBuf,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.debug {
        tracing_subscriber::fmt()
            .with_env_filter("graphwalker_restful=debug,graphwalker_studio=debug")
            .with_target(true)
            .init();
        tracing::debug!("debug logging enabled");
    }

    let mut static_dir = args.static_dir.is_dir().then_some(args.static_dir);
    if static_dir.is_none() {
        let candidates = [
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static"),
            std::env::current_exe()
                .ok()
                .and_then(|e| e.parent().map(|d| d.join("static")))
                .unwrap_or_default(),
        ];
        for candidate in &candidates {
            if candidate.is_dir() {
                static_dir = Some(candidate.clone());
                break;
            }
        }
    }

    let ws_port = args.websocket_port;
    let ws_handle = tokio::spawn(async move {
        if let Err(e) = run_websocket_server(ws_port).await {
            eprintln!("WebSocket server error: {}", e);
        }
    });

    let http_handle = tokio::spawn(async move {
        if let Err(e) = run_http_server(args.browser_port, ws_port, static_dir.as_deref()).await {
            eprintln!("HTTP server error: {}", e);
        }
    });

    eprintln!(
        "GraphWalker Studio\n\
         \x20 Web UI:    http://0.0.0.0:{}\n\
         \x20 WebSocket: ws://0.0.0.0:{}",
        args.browser_port, ws_port,
    );

    tokio::select! {
        _ = ws_handle => {}
        _ = http_handle => {}
    }

    Ok(())
}

struct HttpState {
    index_html: String,
}

async fn serve_index(State(state): State<Arc<HttpState>>) -> Html<String> {
    Html(state.index_html.clone())
}

async fn run_http_server(
    port: u16,
    ws_port: u16,
    static_dir: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    if let Some(static_dir) = static_dir {
        let template = std::fs::read_to_string(static_dir.join("index.html"))?;
        let index_html = template.replace(
            "window.GW_WS_PORT = 9999;",
            &format!("window.GW_WS_PORT = {ws_port};"),
        );
        let state = Arc::new(HttpState { index_html });
        let app = Router::new()
            .route("/", get(serve_index))
            .with_state(state)
            .fallback_service(ServeDir::new(static_dir));
        axum::serve(listener, app).await?;
    } else {
        let app = Router::new()
            .route("/", get(move || serve_embedded_index(ws_port)))
            .fallback(get(serve_embedded_asset));
        axum::serve(listener, app).await?;
    }

    Ok(())
}

async fn serve_embedded_index(ws_port: u16) -> Response {
    let Some(index) = EMBEDDED_STATIC.get_file("index.html") else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let index_html = String::from_utf8_lossy(index.contents()).replace(
        "window.GW_WS_PORT = 9999;",
        &format!("window.GW_WS_PORT = {ws_port};"),
    );
    html_response(index_html)
}

async fn serve_embedded_asset(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let Some(file) = EMBEDDED_STATIC.get_file(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let content_type = match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    };
    (
        [(header::CONTENT_TYPE, HeaderValue::from_static(content_type))],
        file.contents().to_owned(),
    )
        .into_response()
}

fn html_response(index_html: String) -> Response {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )],
        index_html,
    )
        .into_response()
}

async fn run_websocket_server(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let session_mgr = SessionManager::new();
    let app = Router::new()
        .route("/", get(ws_upgrade_handler))
        .with_state(session_mgr);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    State(session_mgr): State<SessionManager>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| graphwalker_restful::websocket::handle_socket(socket, session_mgr))
}
