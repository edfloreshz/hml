use std::{
    fs, io,
    net::SocketAddr,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use axum::{
    Router,
    body::Body,
    extract::{Path as AxumPath, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response, Sse},
    routing::get,
};
use tokio::sync::broadcast;

use hml::CompileDirectoryResult;

const LIVE_RELOAD_PATH: &str = "/__hml/live-reload";
const LIVE_RELOAD_SCRIPT: &str = r#"<script>
(() => {
    const source = new EventSource("/__hml/live-reload");
    source.onmessage = (event) => {
        if (event.data === "reload") {
            window.location.reload();
        }
    };
    source.onerror = () => {
        console.warn("[hml] live reload connection lost");
    };
})();
</script>"#;

#[derive(Clone)]
pub struct DevServer {
    live_reload_tx: broadcast::Sender<String>,
}

#[derive(Clone)]
struct AppState {
    out_dir: Arc<PathBuf>,
    live_reload_tx: broadcast::Sender<String>,
}

impl DevServer {
    pub async fn start(
        out_dir: PathBuf,
        host: &str,
        port: u16,
    ) -> Result<(Self, SocketAddr), String> {
        let listener = tokio::net::TcpListener::bind((host, port))
            .await
            .map_err(|error| format!("failed to bind dev server to {host}:{port}: {error}"))?;

        let address = listener
            .local_addr()
            .map_err(|error| format!("failed to read dev server address: {error}"))?;

        let (live_reload_tx, _) = broadcast::channel(32);
        let state = AppState {
            out_dir: Arc::new(out_dir),
            live_reload_tx: live_reload_tx.clone(),
        };

        let app = Router::new()
            .route(LIVE_RELOAD_PATH, get(live_reload))
            .route("/", get(serve_index))
            .route("/{*path}", get(serve_path))
            .with_state(state);

        tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, app).await {
                eprintln!("dev server error: {error}");
            }
        });

        Ok((Self { live_reload_tx }, address))
    }

    pub fn notify_reload(&self) {
        let _ = self.live_reload_tx.send(String::from("reload"));
    }
}

pub fn inject_live_reload(result: &CompileDirectoryResult) -> Result<(), String> {
    for file in &result.files {
        let html = fs::read_to_string(&file.html_path).map_err(|error| {
            format!(
                "failed to read generated html '{}': {error}",
                file.html_path.display()
            )
        })?;

        let injected = inject_live_reload_script(&html);
        fs::write(&file.html_path, injected).map_err(|error| {
            format!(
                "failed to write generated html '{}': {error}",
                file.html_path.display()
            )
        })?;
    }

    Ok(())
}

fn inject_live_reload_script(html: &str) -> String {
    if html.contains(LIVE_RELOAD_PATH) {
        return html.to_string();
    }

    if let Some(index) = html.rfind("</body>") {
        let mut output = String::with_capacity(html.len() + LIVE_RELOAD_SCRIPT.len() + 1);
        output.push_str(&html[..index]);
        output.push_str(LIVE_RELOAD_SCRIPT);
        output.push('\n');
        output.push_str(&html[index..]);
        return output;
    }

    if let Some(index) = html.rfind("</html>") {
        let mut output = String::with_capacity(html.len() + LIVE_RELOAD_SCRIPT.len() + 1);
        output.push_str(&html[..index]);
        output.push_str(LIVE_RELOAD_SCRIPT);
        output.push('\n');
        output.push_str(&html[index..]);
        return output;
    }

    let mut output = String::with_capacity(html.len() + LIVE_RELOAD_SCRIPT.len() + 1);
    output.push_str(html);
    if !html.ends_with('\n') {
        output.push('\n');
    }
    output.push_str(LIVE_RELOAD_SCRIPT);
    output
}

async fn live_reload(
    State(state): State<AppState>,
) -> Sse<
    impl futures_core::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
> {
    let receiver = state.live_reload_tx.subscribe();
    let stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
        loop {
            match receiver.recv().await {
                Ok(message) => {
                    let event = axum::response::sse::Event::default().data(message);
                    return Some((Ok(event), receiver));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

async fn serve_index(State(state): State<AppState>) -> Response {
    serve_requested_path(state, Path::new("index.html"))
}

async fn serve_path(State(state): State<AppState>, AxumPath(path): AxumPath<String>) -> Response {
    serve_requested_path(state, Path::new(&path))
}

fn serve_requested_path(state: AppState, requested: &Path) -> Response {
    let safe_path = match sanitize_request_path(requested) {
        Some(path) => path,
        None => return status_response(StatusCode::BAD_REQUEST, "invalid path"),
    };

    match resolve_file_path(&state.out_dir, &safe_path) {
        Ok(Some(path)) => match read_response(&path) {
            Ok(response) => response,
            Err(error) => status_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to read '{}': {error}", path.display()),
            ),
        },
        Ok(None) => status_response(StatusCode::NOT_FOUND, "not found"),
        Err(error) => status_response(StatusCode::INTERNAL_SERVER_ERROR, &error),
    }
}

fn resolve_file_path(out_dir: &Path, requested: &Path) -> Result<Option<PathBuf>, String> {
    let direct = out_dir.join(requested);
    if is_file(&direct) {
        return Ok(Some(direct));
    }

    if requested.extension().is_none() {
        let html = out_dir.join(requested).with_extension("html");
        if is_file(&html) {
            return Ok(Some(html));
        }
    }

    let index = out_dir.join(requested).join("index.html");
    if is_file(&index) {
        return Ok(Some(index));
    }

    Ok(None)
}

fn is_file(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.is_file(),
        Err(_) => false,
    }
}

fn sanitize_request_path(requested: &Path) -> Option<PathBuf> {
    let mut output = PathBuf::new();

    for component in requested.components() {
        match component {
            Component::Normal(part) => output.push(part),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => return None,
        }
    }

    Some(output)
}

fn read_response(path: &Path) -> Result<Response, io::Error> {
    let bytes = fs::read(path)?;
    let mime = content_type(path);

    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    Ok(response)
}

fn status_response(status: StatusCode, message: &str) -> Response {
    (status, message.to_string()).into_response()
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
