use tauri::command;
use tauri::Manager;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode, Method};
use hyper::header::{CONTENT_TYPE, ACCESS_CONTROL_ALLOW_ORIGIN};
use std::convert::Infallible;

lazy_static::lazy_static! {
    static ref CACHE_MAP: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref PROXY_PORT: Mutex<Option<u16>> = Mutex::new(None);
}

#[command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[command]
async fn fetch_url_content(url: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let response = client.get(&url).send().await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    let text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(text)
}

async fn fetch_url_bytes(url: String) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let response = client.get(&url).send().await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read bytes: {}", e))?;

    Ok(bytes.to_vec())
}

async fn fetch_url_with_final_url(url: String) -> Result<(Vec<u8>, String), String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .redirect(reqwest::redirect::Policy::default())
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let response = client.get(&url).send().await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    let final_url = response.url().to_string();

    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read bytes: {}", e))?;

    Ok((bytes.to_vec(), final_url))
}

#[command]
async fn proxy_hls_request(url: String) -> Result<Vec<u8>, String> {
    fetch_url_bytes(url).await
}

#[derive(serde::Serialize)]
struct CacheResult {
    success: bool,
    local_url: Option<String>,
    error: Option<String>,
}

#[command]
async fn cache_and_get_local_url(url: String, app_handle: tauri::AppHandle) -> Result<CacheResult, String> {
    let cached_url = {
        let cache_map = CACHE_MAP.lock().await;
        cache_map.get(&url).cloned()
    };

    if let Some(local_path) = cached_url {
        return Ok(CacheResult {
            success: true,
            local_url: Some(local_path),
            error: None,
        });
    }

    let bytes = fetch_url_bytes(url.clone()).await?;

    let cache_dir = app_handle.path().cache_dir()
        .map_err(|e| format!("Failed to get cache dir: {}", e))?;

    let hls_cache_dir = cache_dir.join("hls-cache");
    fs::create_dir_all(&hls_cache_dir).await
        .map_err(|e| format!("Failed to create cache dir: {}", e))?;

    let file_name = format!("{}.dat", url.replace(&['/', ':', '.', '?', '&', '='][..], "_"));
    let file_path = hls_cache_dir.join(file_name);

    let mut file = fs::File::create(&file_path).await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(&bytes).await
        .map_err(|e| format!("Failed to write file: {}", e))?;

    let local_url = format!("asset://{}", file_path.display().to_string().replace("\\", "/"));

    {
        let mut cache_map = CACHE_MAP.lock().await;
        cache_map.insert(url.clone(), local_url.clone());
    }

    Ok(CacheResult {
        success: true,
        local_url: Some(local_url),
        error: None,
    })
}

#[command]
async fn start_hls_proxy_server(app_handle: tauri::AppHandle) -> Result<u16, String> {
    let existing_port = {
        let port_lock = PROXY_PORT.lock().await;
        *port_lock
    };

    if let Some(port) = existing_port {
        return Ok(port);
    }

    let app_handle = Arc::new(app_handle);

    let make_svc = make_service_fn(move |_conn| {
        let app_handle = app_handle.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let app_handle = app_handle.clone();
                async move {
                    handle_proxy_request(req, app_handle).await
                }
            }))
        }
    });

    let addr = ([127, 0, 0, 1], 0).into();
    let server = Server::bind(&addr).serve(make_svc);
    let local_addr = server.local_addr();

    tokio::spawn(async move {
        if let Err(e) = server.await {
            eprintln!("HLS proxy server error: {}", e);
        }
    });

    let port = local_addr.port();
    {
        let mut port_lock = PROXY_PORT.lock().await;
        *port_lock = Some(port);
    }

    println!("HLS proxy server started on port {}", port);
    Ok(port)
}

async fn handle_proxy_request(
    req: Request<Body>,
    _app_handle: Arc<tauri::AppHandle>,
) -> Result<Response<Body>, Infallible> {
    if req.method() == Method::OPTIONS {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .header("Access-Control-Allow-Methods", "GET, OPTIONS")
            .header("Access-Control-Allow-Headers", "*")
            .body(Body::empty())
            .unwrap());
    }

    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("");

    let url_encoded = match path.strip_prefix("/proxy/") {
        Some(encoded) => encoded,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(Body::from("Invalid path format"))
                .unwrap());
        }
    };

    let full_url = match urlencoding::decode(url_encoded) {
        Ok(decoded) => {
            if !query.is_empty() {
                format!("{}?{}", decoded.into_owned(), query)
            } else {
                decoded.into_owned()
            }
        },
        Err(e) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(Body::from(format!("URL decode error: {}", e)))
                .unwrap());
        }
    };

    println!("Proxying request to: {}", full_url);

    let is_m3u8 = full_url.contains(".m3u8") || full_url.contains(".m3u") || full_url.contains("vodId=");

    if is_m3u8 {
        match fetch_url_with_final_url(full_url.clone()).await {
            Ok((bytes, final_url)) => {
                let mut response_body = bytes.clone();

                if let Ok(m3u8_text) = String::from_utf8(bytes) {
                    let base_url = extract_base_url(&final_url);

                    let modified_m3u8 = rewrite_m3u8_urls(&m3u8_text, &base_url);
                    response_body = modified_m3u8.into_bytes();
                    println!("Rewrote m3u8 URLs with base: {}, new size: {} bytes", base_url, response_body.len());
                }

                println!("Successfully fetched {} bytes", response_body.len());

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "application/vnd.apple.mpegurl")
                    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .header("Cache-Control", "no-cache, no-store, must-revalidate")
                    .header("Pragma", "no-cache")
                    .header("Expires", "0")
                    .body(Body::from(response_body))
                    .unwrap())
            }
            Err(e) => {
                eprintln!("Proxy fetch error: {}", e);
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .body(Body::from(format!("Proxy error: {}", e)))
                    .unwrap())
            }
        }
    } else {
        match fetch_url_bytes(full_url.clone()).await {
            Ok(bytes) => {
                let content_type = if full_url.contains(".ts") {
                    "video/MP2T"
                } else {
                    "application/octet-stream"
                };

                println!("Successfully fetched {} bytes", bytes.len());

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, content_type)
                    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .header("Cache-Control", "public, max-age=3600")
                    .body(Body::from(bytes))
                    .unwrap())
            }
            Err(e) => {
                eprintln!("Proxy fetch error: {}", e);
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .body(Body::from(format!("Proxy error: {}", e)))
                    .unwrap())
            }
        }
    }
}

fn extract_base_url(final_url: &str) -> String {
    let url_without_query = if let Some(pos) = final_url.find("?") {
        &final_url[..pos]
    } else {
        final_url
    };

    if let Some(pos) = url_without_query.rfind("/") {
        url_without_query[..pos + 1].to_string()
    } else {
        final_url.to_string()
    }
}

fn rewrite_m3u8_urls(m3u8_content: &str, base_url: &str) -> String {
    let mut result = String::new();

    for line in m3u8_content.lines() {
        if line.starts_with('#') || line.is_empty() {
            result.push_str(line);
            result.push('\n');
        } else {
            let segment_url = if line.starts_with("http://") || line.starts_with("https://") {
                line.to_string()
            } else if line.starts_with("/") {
                if let Some(slash_pos) = base_url.find("//") {
                    if let Some(third_slash) = base_url[slash_pos + 2..].find("/") {
                        let domain_part = &base_url[..slash_pos + 2 + third_slash];
                        format!("{}{}", domain_part, line)
                    } else {
                        format!("{}{}", base_url, line)
                    }
                } else {
                    format!("{}{}", base_url, line)
                }
            } else {
                format!("{}{}", base_url, line)
            };

            let encoded_url = urlencoding::encode(&segment_url);
            result.push_str(&format!("/proxy/{}\n", encoded_url));
        }
    }

    result
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            fetch_url_content,
            proxy_hls_request,
            cache_and_get_local_url,
            start_hls_proxy_server
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

