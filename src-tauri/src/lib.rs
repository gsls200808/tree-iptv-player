use tauri::command;
use tauri::Manager;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use std::path::PathBuf;
use std::sync::Arc;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode, Method};
use hyper::header::{CONTENT_TYPE, ACCESS_CONTROL_ALLOW_ORIGIN};
use std::convert::Infallible;

struct M3u8Session {
    redirect_chain: Vec<String>,
    last_host: String,
    last_base_url: String,
    #[allow(dead_code)]
    timestamp: std::time::Instant,
    #[allow(dead_code)]
    media_sequence: Option<u64>,
}

lazy_static::lazy_static! {
    static ref CACHE_MAP: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref PROXY_PORT: Mutex<Option<u16>> = Mutex::new(None);
    static ref M3U8_SESSION_MAP: Mutex<HashMap<String, Vec<M3u8Session>>> = Mutex::new(HashMap::new());
    static ref TS_TO_M3U8_MAP: Mutex<HashMap<String, (String, u64)>> = Mutex::new(HashMap::new());
    static ref ACTIVE_HOST_MAP: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref RTSP_PROCESSES: Mutex<HashMap<String, tokio::process::Child>> = Mutex::new(HashMap::new());
    static ref RTSP_PORTS: Mutex<HashMap<String, u16>> = Mutex::new(HashMap::new());
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

/// Resolve a stream URL that may 302 redirect (e.g. vodId URLs).
/// Returns the final URL after following all redirects.
#[command]
async fn resolve_stream_url(url: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let response = client.get(&url).send().await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    let final_url = response.url().to_string();
    println!("Resolved: {} -> {}", url, final_url);
    Ok(final_url)
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

async fn fetch_with_redirect_chain(url: String) -> Result<(Vec<u8>, Vec<String>), String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let mut redirect_chain = Vec::new();
    let mut current_url = url.clone();

    loop {
        redirect_chain.push(current_url.clone());

        let response = client.get(&current_url).send().await
            .map_err(|e| format!("Failed to fetch URL: {}", e))?;

        let status = response.status();

        if status.is_redirection() {
            if let Some(location) = response.headers().get("location") {
                let location_str = location.to_str()
                    .map_err(|e| format!("Invalid location header: {}", e))?;

                let next_url = if location_str.starts_with("http://") || location_str.starts_with("https://") {
                    location_str.to_string()
                } else {
                    let base_url = extract_base_url(&current_url);
                    if location_str.starts_with("/") {
                        if let Some(slash_pos) = base_url.find("//") {
                            if let Some(third_slash) = base_url[slash_pos + 2..].find("/") {
                                let domain_part = &base_url[..slash_pos + 2 + third_slash];
                                format!("{}{}", domain_part, location_str)
                            } else {
                                format!("{}{}", base_url, location_str)
                            }
                        } else {
                            format!("{}{}", base_url, location_str)
                        }
                    } else {
                        format!("{}{}", base_url, location_str)
                    }
                };

                current_url = next_url;
            } else {
                return Err("Redirect without location header".to_string());
            }
        } else {
            let final_bytes = response.bytes().await
                .map_err(|e| format!("Failed to read bytes: {}", e))?
                .to_vec();
            return Ok((final_bytes, redirect_chain));
        }
    }
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

    let is_m3u8 = full_url.contains(".m3u8") || full_url.contains(".m3u");

    if is_m3u8 {
        // HLS m3u8: buffer and rewrite URLs
        match handle_m3u8_request(full_url.clone()).await {
            Ok((response_body, redirect_chain)) => {
                println!("Successfully fetched {} bytes", response_body.len());

                println!("\n========== M3U8 Redirect Chain ==========");
                for (i, url) in redirect_chain.iter().enumerate() {
                    let host = extract_host_from_url(url);
                    println!("  [{}] {} (Host: {})", i + 1, url, host);
                }
                println!("==========================================\n");

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
        // For non-m3u8 URLs: follow redirects to detect actual content type
        // (handles vodId URLs that redirect to .flv or .m3u8)
        let client = match reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .body(Body::from(format!("Failed to create client: {}", e)))
                    .unwrap());
            }
        };

        let mut check_url = full_url.clone();
        let mut is_final_flv = false;

        // Follow redirects to find the final URL
        for _ in 0..10 {
            let resp = match client.get(&check_url).send().await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Redirect check failed: {}", e);
                    break;
                }
            };

            if resp.status().is_redirection() {
                if let Some(location) = resp.headers().get("location") {
                    let loc = location.to_str().unwrap_or("");
                    check_url = if loc.starts_with("http") {
                        loc.to_string()
                    } else {
                        let base = extract_base_url(&check_url);
                        format!("{}{}", base, loc)
                    };
                    if check_url.contains(".flv") {
                        is_final_flv = true;
                        break;
                    }
                    if check_url.contains(".m3u8") || check_url.contains(".m3u") {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                // Check content-type as fallback
                if let Some(ct) = resp.headers().get("content-type") {
                    let ct_str = ct.to_str().unwrap_or("");
                    if ct_str.contains("video/x-flv") {
                        is_final_flv = true;
                    }
                }
                break;
            }
        }

        if is_final_flv || check_url.contains(".flv") {
            // FLV stream: proxy with proper headers
            match handle_flv_proxy_request(full_url.clone()).await {
                Ok(response) => {
                    println!("FLV streaming started for: {}", full_url);
                    Ok(response)
                }
                Err(e) => {
                    eprintln!("FLV proxy error: {}", e);
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                        .body(Body::from(format!("FLV proxy error: {}", e)))
                        .unwrap())
                }
            }
        } else if check_url.contains(".m3u8") || check_url.contains(".m3u") {
            // Redirected to m3u8: handle as HLS
            match handle_m3u8_request(check_url.clone()).await {
                Ok((response_body, _)) => {
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(CONTENT_TYPE, "application/vnd.apple.mpegurl")
                        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                        .header("Cache-Control", "no-cache")
                        .body(Body::from(response_body))
                        .unwrap())
                }
                Err(e) => {
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                        .body(Body::from(format!("Proxy error: {}", e)))
                        .unwrap())
                }
            }
        } else {
            // Regular TS segment
            match handle_ts_request(full_url.clone()).await {
            Ok(bytes) => {
                let content_type = if full_url.contains(".ts") {
                    "video/MP2T"
                } else {
                    "application/octet-stream"
                };

                println!("Successfully fetched {} bytes", bytes.len());

                let ts_host = extract_host_from_url(&full_url);
                println!("\n========== TS Request ==========");
                println!("  URL: {}", full_url);
                println!("  Host: {}", ts_host);
                println!("================================\n");

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
        }  // close: else (TS handler)
    }  // close: outer if/else (m3u8 vs other)
}

async fn handle_flv_proxy_request(original_url: String) -> Result<Response<Body>, String> {
    // Step 1: Resolve redirect (vodId -> final CDN URL)
    let redirect_client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    let mut current_url = original_url.clone();
    for _ in 0..10 {
        let resp = redirect_client.get(&current_url).send().await
            .map_err(|e| format!("Redirect fetch failed: {}", e))?;
        if resp.status().is_redirection() {
            if let Some(loc) = resp.headers().get("location") {
                let loc_str = loc.to_str().unwrap_or("");
                current_url = if loc_str.starts_with("http") {
                    loc_str.to_string()
                } else {
                    format!("{}{}", extract_base_url(&current_url), loc_str)
                };
            } else {
                break;
            }
        } else {
            break;
        }
    }

    println!("\n========== FLV Stream (curl subprocess) ==========");
    println!("  Original: {}", original_url);
    println!("  Final:    {}", current_url);
    println!("==================================================\n");

    // Spawn curl to stream the FLV data (curl handles Connection: Close correctly)
    let mut child = tokio::process::Command::new("curl")
        .arg("-s")
        .arg("-N")
        .arg(&current_url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn curl: {}", e))?;

    let stdout = child.stdout.take()
        .ok_or("Failed to get curl stdout")?;

    let (mut tx, body) = Body::channel();

    tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut stdout = stdout;
        let mut buf = vec![0u8; 65536];
        loop {
            match stdout.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send_data(bytes::Bytes::copy_from_slice(&buf[..n])).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("FLV curl read error: {}", e);
                    break;
                }
            }
        }
        child.kill().await.ok();
        println!("FLV stream ended for: {}", original_url);
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "video/x-flv")
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header("Cache-Control", "no-cache")
        .header("Transfer-Encoding", "chunked")
        .body(body)
        .unwrap())
}

async fn handle_m3u8_request(original_url: String) -> Result<(Vec<u8>, Vec<String>), String> {
    let (bytes, redirect_chain) = fetch_with_redirect_chain(original_url.clone()).await?;

    let final_url = redirect_chain.last().unwrap_or(&original_url).clone();
    let mut response_body = bytes.clone();

    if let Ok(m3u8_text) = String::from_utf8(bytes) {
        let base_url = extract_base_url(&final_url);
        let host_name = extract_host_from_url(&final_url);

        println!("\n========== M3U8 Processing ==========");
        println!("  Original URL: {}", original_url);
        println!("  Final URL after redirect: {}", final_url);
        println!("  Current Host: {}", host_name);

        let active_host = {
            let active_map = ACTIVE_HOST_MAP.lock().await;
            active_map.get(&original_url).cloned()
        };

        let (use_base_url, use_host_name) = if let Some(ref active) = active_host {
            println!("  ✓ Found active host: {}", active);

            let session_map = M3U8_SESSION_MAP.lock().await;
            if let Some(sessions) = session_map.get(&original_url) {
                let mut found_session = None;
                for session in sessions.iter().rev() {
                    if session.last_host == *active {
                        found_session = Some(session.last_base_url.clone());
                        break;
                    }
                }

                if let Some(active_base_url) = found_session {
                    println!("  ✓ Using active host's base URL: {}", active_base_url);
                    (active_base_url, active.clone())
                } else {
                    println!("  ⚠ Active host not in session history, using current");
                    (base_url.clone(), host_name.clone())
                }
            } else {
                println!("  ⚠ No session history, using current");
                (base_url.clone(), host_name.clone())
            }
        } else {
            println!("  ℹ No active host recorded, using current");
            (base_url.clone(), host_name.clone())
        };

        println!("  Final Base URL: {}", use_base_url);
        println!("  Final Host: {}", use_host_name);
        println!("=====================================\n");

        let media_sequence = extract_media_sequence(&m3u8_text);

        let modified_m3u8 = rewrite_m3u8_urls_with_host(&m3u8_text, &use_base_url, &use_host_name, &original_url, media_sequence).await;
        response_body = modified_m3u8.into_bytes();
        println!("Rewrote m3u8 URLs with base: {}, host: {}, seq: {:?}, new size: {} bytes", use_base_url, use_host_name, media_sequence, response_body.len());

        let new_session = M3u8Session {
            redirect_chain: redirect_chain.clone(),
            last_host: host_name,
            last_base_url: base_url,
            timestamp: std::time::Instant::now(),
            media_sequence,
        };

        let mut session_map = M3U8_SESSION_MAP.lock().await;
        let sessions = session_map.entry(original_url.clone()).or_insert_with(Vec::new);

        sessions.push(new_session);

        if sessions.len() > 10 {
            sessions.remove(0);
        }

        println!("Session history for {}: {} entries", original_url, sessions.len());
    }

    Ok((response_body, redirect_chain))
}

async fn handle_ts_request(ts_url: String) -> Result<Vec<u8>, String> {
    let ts_info = TS_TO_M3U8_MAP.lock().await;
    let info = ts_info.get(&ts_url).cloned();
    drop(ts_info);

    let actual_host = extract_host_from_url(&ts_url);
    println!("\n========== TS Request Analysis ==========");
    println!("  TS URL: {}", ts_url);
    println!("  Actual Host: {}", actual_host);

    let m3u8_url_for_active = if let Some((ref m3u8_url, _expected_seq)) = info {
        println!("  Associated M3U8: {}", m3u8_url);

        let active_host = {
            let active_map = ACTIVE_HOST_MAP.lock().await;
            active_map.get(m3u8_url).cloned()
        };

        if let Some(ref active) = active_host {
            if *active != actual_host {
                println!("  ⚠ Host mismatch! Active={}, Actual={}", active, actual_host);
                println!("  Attempting to switch to active host...");

                let filename = extract_filename_from_url(&ts_url);
                let query_start = ts_url.find("?");
                let query_string = if let Some(pos) = query_start {
                    &ts_url[pos..]
                } else {
                    ""
                };

                if let Some(filename) = filename {
                    let session_map = M3U8_SESSION_MAP.lock().await;
                    if let Some(sessions) = session_map.get(m3u8_url) {
                        for session in sessions.iter().rev() {
                            if session.last_host == *active {
                                let active_base_url = &session.last_base_url;
                                let switched_url = format!("{}{}{}", active_base_url, filename, query_string);

                                println!("  Switching to active host URL: {}", switched_url);

                                match fetch_url_bytes(switched_url.clone()).await {
                                    Ok(bytes) => {
                                        println!("  ✓ Switch successful!");
                                        println!("==========================================\n");
                                        return Ok(bytes);
                                    }
                                    Err(e) => {
                                        println!("  ✗ Switch failed: {}, will try original", e);
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            } else {
                println!("  ✓ Host matches active host");
            }
        }

        let session_map = M3U8_SESSION_MAP.lock().await;
        if let Some(sessions) = session_map.get(m3u8_url) {
            println!("  Available Sessions: {}", sessions.len());

            let mut found_match = false;

            for (idx, session) in sessions.iter().enumerate().rev() {
                println!("    Session [{}]: Host={}, BaseURL={}",
                    idx, session.last_host, session.last_base_url);

                if session.last_host == actual_host {
                    found_match = true;
                    println!("    ✓ Host match found at session [{}]", idx);
                    break;
                }
            }

            if !found_match {
                println!("  ⚠ TS host NOT in current session history");
                println!("  Attempting to find fallback from previous sessions...");

                drop(session_map);

                if let Some(fallback_url) = find_fallback_url_for_ts(&ts_url).await {
                    println!("  ✓ Fallback successful! Using: {}", fallback_url);

                    let fallback_host = extract_host_from_url(&fallback_url);
                    let mut active_map = ACTIVE_HOST_MAP.lock().await;
                    active_map.insert(m3u8_url.clone(), fallback_host);
                    drop(active_map);

                    println!("==========================================\n");
                    return fetch_url_bytes(fallback_url).await;
                } else {
                    println!("  ✗ Fallback failed, will try original URL");
                }
            } else {
                println!("  ✓ Using current host (no fallback needed)");

                let mut active_map = ACTIVE_HOST_MAP.lock().await;
                active_map.insert(m3u8_url.clone(), actual_host.clone());
                drop(active_map);
            }
        } else {
            println!("  ⚠ No session history found for this M3U8");
        }

        Some(m3u8_url.clone())
    } else {
        println!("  ⚠ No M3U8 association found for this TS");
        None
    };

    println!("==========================================\n");

    let result = fetch_url_bytes(ts_url.clone()).await;

    if result.is_ok() {
        if let Some(ref m3u8_url) = m3u8_url_for_active {
            let mut active_map = ACTIVE_HOST_MAP.lock().await;
            active_map.insert(m3u8_url.clone(), actual_host.clone());
            drop(active_map);
        }
    }

    if result.is_err() {
        println!("TS fetch failed for {}, trying fallback logic", ts_url);

        if let Some(fallback_url) = find_fallback_url_for_ts(&ts_url).await {
            println!("Trying fallback URL: {}", fallback_url);

            let fallback_host = extract_host_from_url(&fallback_url);
            if let Some(ref m3u8_url) = m3u8_url_for_active {
                let mut active_map = ACTIVE_HOST_MAP.lock().await;
                active_map.insert(m3u8_url.clone(), fallback_host);
                drop(active_map);
            }

            return fetch_url_bytes(fallback_url).await;
        }
    }

    result
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

fn extract_host_from_url(url: &str) -> String {
    let url_without_query = if let Some(pos) = url.find("?") {
        &url[..pos]
    } else {
        url
    };

    if let Some(start) = url_without_query.find("://") {
        let after_protocol = &url_without_query[start + 3..];
        if let Some(end) = after_protocol.find("/") {
            after_protocol[..end].to_string()
        } else {
            after_protocol.to_string()
        }
    } else {
        url_without_query.to_string()
    }
}
fn extract_media_sequence(m3u8_content: &str) -> Option<u64> {
    for line in m3u8_content.lines() {
        if line.starts_with("#EXT-X-MEDIA-SEQUENCE:") {
            if let Some(seq_str) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
                return seq_str.trim().parse::<u64>().ok();
            }
        }
    }
    None
}

async fn rewrite_m3u8_urls_with_host(m3u8_content: &str, base_url: &str, _host_name: &str, original_m3u8_url: &str, media_sequence: Option<u64>) -> String {
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

            if let Some(seq) = media_sequence {
                let mut ts_to_m3u8 = TS_TO_M3U8_MAP.lock().await;
                ts_to_m3u8.insert(segment_url.clone(), (original_m3u8_url.to_string(), seq));
                drop(ts_to_m3u8);
            }

            let encoded_url = urlencoding::encode(&segment_url);
            result.push_str(&format!("/proxy/{}\n", encoded_url));
        }
    }

    result
}

async fn find_fallback_url_for_ts(ts_url: &str) -> Option<String> {
    let ts_info = TS_TO_M3U8_MAP.lock().await;
    let info = ts_info.get(ts_url).cloned();
    drop(ts_info);

    let (original_m3u8_url, _expected_seq) = info?;

    let session_map = M3U8_SESSION_MAP.lock().await;
    let sessions = session_map.get(&original_m3u8_url)?;

    println!("Found {} session entries for m3u8: {}", sessions.len(), original_m3u8_url);

    let filename = extract_filename_from_url(ts_url)?;

    let query_start = ts_url.find("?");
    let query_string = if let Some(pos) = query_start {
        &ts_url[pos..]
    } else {
        ""
    };

    let current_host = extract_host_from_url(ts_url);
    println!("Current TS host: {}, looking for alternative...", current_host);

    for (session_idx, session) in sessions.iter().enumerate().rev() {
        let redirect_chain = &session.redirect_chain;
        let current_base_url = &session.last_base_url;

        println!("Checking session [{}]: Host={}", session_idx, session.last_host);

        for (chain_idx, chain_url) in redirect_chain.iter().enumerate().rev() {
            let chain_base_url = extract_base_url(chain_url);
            let chain_host = extract_host_from_url(chain_url);

            if chain_base_url == *current_base_url {
                println!("  Chain [{}] skipped (same as current base): {}", chain_idx, chain_host);
                continue;
            }

            if chain_host == current_host {
                println!("  Chain [{}] skipped (same as current host): {}", chain_idx, chain_host);
                continue;
            }

            let fallback_url = format!("{}{}{}", chain_base_url, filename, query_string);
            println!("  Trying fallback to chain [{}] host {}: {}", chain_idx, chain_host, fallback_url);

            match fetch_url_bytes(fallback_url.clone()).await {
                Ok(_) => {
                    println!("  ✓ Fallback successful to host: {}", chain_host);
                    return Some(fallback_url);
                }
                Err(e) => {
                    println!("  ✗ Fallback failed: {}", e);
                }
            }
        }
    }

    println!("All fallback attempts failed");
    None
}


fn extract_filename_from_url(url: &str) -> Option<String> {
    let url_without_query = if let Some(pos) = url.find("?") {
        &url[..pos]
    } else {
        url
    };

    if let Some(pos) = url_without_query.rfind("/") {
        Some(url_without_query[pos + 1..].to_string())
    } else {
        None
    }
}

async fn serve_hls_file(
    req: Request<Body>,
    hls_dir: Arc<PathBuf>,
) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path().trim_start_matches('/');
    let file_path = if path.is_empty() {
        hls_dir.join("playlist.m3u8")
    } else {
        hls_dir.join(path)
    };

    match tokio::fs::read(&file_path).await {
        Ok(contents) => {
            let ct = if file_path.extension().map_or(false, |e| e == "m3u8") {
                "application/vnd.apple.mpegurl"
            } else {
                "video/mp2t"
            };
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, ct)
                .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .header("Cache-Control", "no-cache")
                .body(Body::from(contents))
                .unwrap())
        }
        Err(_) => {
            // Retry for m3u8 (ffmpeg may not have created it yet)
            if path.is_empty() || path.ends_with(".m3u8") {
                for _ in 0..10 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    if let Ok(contents) = tokio::fs::read(&file_path).await {
                        return Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(CONTENT_TYPE, "application/vnd.apple.mpegurl")
                            .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                            .header("Cache-Control", "no-cache")
                            .body(Body::from(contents))
                            .unwrap());
                    }
                }
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .body(Body::from("m3u8 not ready, ffmpeg may have failed"))
                    .unwrap())
            } else {
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .body(Body::from("File not found"))
                    .unwrap())
            }
        }
    }
}

#[command]
async fn start_rtsp_proxy(rtsp_url: String) -> Result<u16, String> {
    // Check if already running
    {
        let ports = RTSP_PORTS.lock().await;
        if let Some(&port) = ports.get(&rtsp_url) {
            return Ok(port);
        }
    }

    // Stop any existing process for this URL
    stop_rtsp_proxy_internal(&rtsp_url).await;

    // Create temp directory for HLS output
    let hls_dir = std::env::temp_dir().join(format!(
        "rtsp_hls_{}",
        rtsp_url.replace(&['/', ':', '.', '?', '&', '='][..], "_")
    ));
    tokio::fs::create_dir_all(&hls_dir)
        .await
        .map_err(|e| format!("Failed to create HLS dir: {}", e))?;

    let playlist_path = hls_dir.join("playlist.m3u8");
    let segment_pattern = hls_dir.join("stream%05d.ts");

    println!("RTSP proxy: dir={}", hls_dir.display());

    // Start FFmpeg: RTSP -> HLS (TS segments)
    let child = tokio::process::Command::new("ffmpeg")
        .arg("-rtsp_transport")
        .arg("tcp")
        .arg("-i")
        .arg(&rtsp_url)
        .arg("-c:v")
        .arg("copy")
        .arg("-c:a")
        .arg("aac")
        .arg("-f")
        .arg("hls")
        .arg("-hls_time")
        .arg("2")
        .arg("-hls_list_size")
        .arg("5")
        .arg("-hls_flags")
        .arg("delete_segments")
        .arg("-hls_segment_type")
        .arg("mpegts")
        .arg("-hls_segment_filename")
        .arg(segment_pattern.to_str().unwrap())
        .arg(playlist_path.to_str().unwrap())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn ffmpeg: {}. Is ffmpeg installed?", e))?;

    // Start HTTP server for HLS files
    let server_dir = Arc::new(hls_dir);
    let make_svc = make_service_fn(move |_conn| {
        let dir = server_dir.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                serve_hls_file(req, dir.clone())
            }))
        }
    });

    let addr = ([127, 0, 0, 1], 0).into();
    let server = Server::bind(&addr).serve(make_svc);
    let port = server.local_addr().port();

    tokio::spawn(async move {
        if let Err(e) = server.await {
            eprintln!("RTSP HLS server error: {}", e);
        }
    });

    // Store process and port
    {
        let mut processes = RTSP_PROCESSES.lock().await;
        processes.insert(rtsp_url.clone(), child);
    }
    {
        let mut ports = RTSP_PORTS.lock().await;
        ports.insert(rtsp_url.clone(), port);
    }

    println!("RTSP proxy ready: {} -> http://127.0.0.1:{}/", rtsp_url, port);
    Ok(port)
}

async fn stop_rtsp_proxy_internal(rtsp_url: &str) {
    let mut processes = RTSP_PROCESSES.lock().await;
    if let Some(mut child) = processes.remove(rtsp_url) {
        let _ = child.start_kill();
        println!("Killed FFmpeg for: {}", rtsp_url);
    }
    drop(processes);

    let mut ports = RTSP_PORTS.lock().await;
    ports.remove(rtsp_url);
    drop(ports);

    // Clean up temp directory
    let hls_dir = std::env::temp_dir().join(format!(
        "rtsp_hls_{}",
        rtsp_url.replace(&['/', ':', '.', '?', '&', '='][..], "_")
    ));
    let _ = tokio::fs::remove_dir_all(&hls_dir).await;
}

#[command]
async fn stop_rtsp_proxy(rtsp_url: String) -> Result<(), String> {
    stop_rtsp_proxy_internal(&rtsp_url).await;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            fetch_url_content,
            resolve_stream_url,
            proxy_hls_request,
            cache_and_get_local_url,
            start_hls_proxy_server,
            start_rtsp_proxy,
            stop_rtsp_proxy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}