use axum::body::Body;
use axum::http::header::{
    HeaderName, HeaderValue, ALLOW, CONTENT_LENGTH, CONTENT_TYPE, ORIGIN, VARY,
};
use axum::http::uri::{PathAndQuery, Uri};
use axum::http::{Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use std::task::{Context, Poll};
use tower::Service;

static ACCESS_CONTROL_REQUEST_METHOD: HeaderName =
    HeaderName::from_static("access-control-request-method");
static ACCESS_CONTROL_REQUEST_HEADERS: HeaderName =
    HeaderName::from_static("access-control-request-headers");
static ACCESS_CONTROL_ALLOW_ORIGIN: HeaderName =
    HeaderName::from_static("access-control-allow-origin");
static ACCESS_CONTROL_ALLOW_METHODS: HeaderName =
    HeaderName::from_static("access-control-allow-methods");
static ACCESS_CONTROL_ALLOW_HEADERS: HeaderName =
    HeaderName::from_static("access-control-allow-headers");
static ACCESS_CONTROL_MAX_AGE: HeaderName = HeaderName::from_static("access-control-max-age");

const ALLOW_METHODS: &str = "DELETE, GET, HEAD, OPTIONS, PATCH, POST, PUT, QUERY";
const PREFLIGHT_VARY: &str = "Origin, Access-Control-Request-Method, \
     Access-Control-Request-Headers, Access-Control-Request-Private-Network";

fn json_response(status: StatusCode, body: &'static str) -> Response {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("response builder")
}

fn finalize(mut res: Response, has_origin: bool) -> Response {
    let headers = res.headers_mut();
    match headers.get(VARY) {
        None => {
            headers.insert(VARY, HeaderValue::from_static("Origin"));
        }
        Some(existing) => {
            if let Ok(text) = existing.to_str() {
                if !text.split(',').any(|part| part.trim() == "Origin") {
                    let merged = format!("{text}, Origin");
                    if let Ok(value) = HeaderValue::from_str(&merged) {
                        headers.insert(VARY, value);
                    }
                }
            }
        }
    }
    if has_origin {
        headers.insert(&ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    }
    res
}

pub async fn global_middleware(req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let has_origin = req.headers().contains_key(ORIGIN);

    // Starlette no enruta rutas que contengan "//" (patron [^/]+).
    if path.contains("//") {
        return finalize(
            json_response(StatusCode::NOT_FOUND, r#"{"detail":"Not Found"}"#),
            has_origin,
        );
    }

    // StaticFiles no sirve el directorio raiz del mount.
    if path == "/static" || path == "/static/" {
        return finalize(
            json_response(StatusCode::NOT_FOUND, r#"{"detail":"Not Found"}"#),
            has_origin,
        );
    }

    // Starlette solo permite HEAD explicitamente si la ruta lo define (ej: static).
    // Para rutas GET normales sin HEAD explicito, FastAPI responde 405 Method Not Allowed.
    if req.method() == Method::HEAD && !path.starts_with("/static/") {
        return finalize(
            json_response(
                StatusCode::METHOD_NOT_ALLOWED,
                r#"{"detail":"Method Not Allowed"}"#,
            ),
            has_origin,
        );
    }

    let is_preflight = req.method() == Method::OPTIONS
        && has_origin
        && req.headers().contains_key(&ACCESS_CONTROL_REQUEST_METHOD);

    if is_preflight {
        let mut res = Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .header(&ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .header(&ACCESS_CONTROL_ALLOW_METHODS, ALLOW_METHODS)
            .header(&ACCESS_CONTROL_MAX_AGE, "600")
            .header(VARY, PREFLIGHT_VARY)
            .body(Body::from("OK"))
            .expect("preflight builder");
        if let Some(value) = req.headers().get(&ACCESS_CONTROL_REQUEST_HEADERS) {
            res.headers_mut()
                .insert(&ACCESS_CONTROL_ALLOW_HEADERS, value.clone());
        }
        return res;
    }

    let res = next.run(req).await;

    // Starlette HTTPException(405) sin cabecera Allow y con cuerpo JSON.
    let mut res = res;
    if res.status() == StatusCode::METHOD_NOT_ALLOWED {
        res.headers_mut().remove(ALLOW);
        res.headers_mut().remove(CONTENT_LENGTH);
        res.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        *res.body_mut() = Body::from(r#"{"detail":"Method Not Allowed"}"#);
    }

    finalize(res, has_origin)
}

#[derive(Clone)]
pub struct OuterService<S> {
    inner: S,
}

impl<S> OuterService<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> Service<Request<Body>> for OuterService<S>
where
    S: Service<Request<Body>, Response = Response, Error = std::convert::Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = std::convert::Infallible;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let path = req.uri().path().to_string();
        if !path.contains("//") && path.len() > 1 && path.ends_with('/') {
            let new_path = &path[..path.len() - 1];
            let query = req
                .uri()
                .query()
                .map(|q| format!("?{q}"))
                .unwrap_or_default();
            let target = format!("{new_path}{query}");
            if let Ok(pq) = PathAndQuery::try_from(target) {
                let mut parts = req.uri().clone().into_parts();
                parts.path_and_query = Some(pq);
                if let Ok(uri) = Uri::from_parts(parts) {
                    *req.uri_mut() = uri;
                }
            }
        }
        self.inner.call(req)
    }
}
