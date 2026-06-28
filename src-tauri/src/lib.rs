use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
mod rpc;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Account {
    Offline {
        username: String,
    },
    Ely {
        username: String,
        uuid: String,
        access_token: String,
    },
    Microsoft {
        username: String,
        uuid: String,
        access_token: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlaySession {
    pub start: u64,
    pub duration: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlaytimeStats {
    pub sessions: Vec<PlaySession>,
}

impl Account {
    #[allow(dead_code)]
    fn username(&self) -> &str {
        match self {
            Account::Offline { username } => username,
            Account::Ely { username, .. } => username,
            Account::Microsoft { username, .. } => username,
        }
    }
}

// ─── Mojang API structs ────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
struct VersionManifest {
    versions: Vec<VersionEntry>,
}

#[derive(Deserialize, Clone, Serialize)]
struct VersionEntry {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
    url: String,
}

#[derive(Serialize, Clone)]
struct VersionInfo {
    id: String,
    installed: bool,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct VersionJson {
    #[serde(rename = "mainClass")]
    main_class: String,
    jar: Option<String>,
    #[serde(rename = "inheritsFrom")]
    inherits_from: Option<String>,
    libraries: Vec<Library>,
    downloads: Option<Downloads>,
    #[serde(rename = "assetIndex")]
    asset_index: Option<AssetIndexInfo>,
    assets: Option<String>,
    arguments: Option<Arguments>,
    #[serde(rename = "minecraftArguments")]
    minecraft_arguments: Option<String>,
}

#[derive(Deserialize)]
struct Downloads {
    client: FileDownload,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct FileDownload {
    url: String,
    sha1: String,
    size: u64,
}

#[derive(Deserialize)]
struct Library {
    name: String,
    url: Option<String>,
    downloads: Option<LibraryDownloads>,
    rules: Option<Vec<Rule>>,
    natives: Option<HashMap<String, String>>,
    extract: Option<ExtractInfo>,
}

#[derive(Deserialize)]
struct LibraryDownloads {
    artifact: Option<LibraryArtifact>,
    classifiers: Option<HashMap<String, LibraryArtifact>>,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone)]
struct LibraryArtifact {
    path: String,
    url: String,
    sha1: String,
    size: u64,
}

#[derive(Deserialize)]
struct Rule {
    action: String,
    os: Option<OsRule>,
}

#[derive(Deserialize)]
struct OsRule {
    name: Option<String>,
}

#[derive(Deserialize)]
struct ExtractInfo {
    exclude: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct AssetIndexInfo {
    id: String,
    url: String,
    sha1: String,
    size: u64,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Arguments {
    game: Option<Vec<serde_json::Value>>,
    jvm: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
struct AssetIndex {
    objects: HashMap<String, AssetObject>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

// ─── Progress event ────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
struct ProgressEvent {
    stage: String,
    current: u64,
    total: u64,
    message: String,
}

// ─── Update structs ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
}

#[tauri::command]
fn set_rpc_activity(rpc: tauri::State<Arc<rpc::Rpc>>, details: String, state: String) {
    rpc.set_activity(&details, &state);
}

#[tauri::command]
fn clear_rpc(rpc: tauri::State<Arc<rpc::Rpc>>) {
    rpc.clear();
}

#[tauri::command]
async fn check_for_updates(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    log_update("Checking for updates...");
    let client = reqwest::Client::new();
    let response = client
        .get("https://raw.githubusercontent.com/Conteek/femboy-launcher/main/latest.json")
        .send()
        .await
        .map_err(|e| {
            let err = format!("Failed to fetch latest.json: {}", e);
            log_update(&err);
            err
        })?;

    if !response.status().is_success() {
        let err = "Failed to fetch latest.json".to_string();
        log_update(&err);
        return Err(err);
    }

    let update_info: UpdateInfo = response.json().await.map_err(|e| {
        let err = format!("Failed to parse update info: {}", e);
        log_update(&err);
        err
    })?;

    // Retrieve version dynamically from Tauri context
    let current_version = app.package_info().version.to_string();

    if update_info.version != current_version {
        log_update(&format!(
            "Update found: {} (current: {})",
            update_info.version, current_version
        ));
        Ok(Some(update_info))
    } else {
        log_update("No update available.");
        Ok(None)
    }
}

#[tauri::command]
async fn download_update(app: AppHandle, url: String) -> Result<String, String> {
    log_update(&format!("Downloading update from: {}", url));
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.map_err(|e| {
        let err = format!("Failed to download update: {}", e);
        log_update(&err);
        err
    })?;

    if !response.status().is_success() {
        let err = "Failed to download update".to_string();
        log_update(&err);
        return Err(err);
    }

    let total_size = response.content_length().unwrap_or(0);

    let temp_dir = std::env::temp_dir().join("femboy_update");
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let dest = temp_dir.join("app.exe.gz");
    let mut file = fs::File::create(&dest).map_err(|e| e.to_string())?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            emit_progress(
                &app,
                "download",
                downloaded,
                total_size,
                &format!("Скачивание: {}/{}", downloaded, total_size),
            );
        }
    }

    log_update(&format!("Update downloaded to: {:?}", dest));
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
async fn apply_update(app: AppHandle, temp_archive_path: String) -> Result<(), String> {
    log_update(&format!("Applying update from: {}", temp_archive_path));
    use tauri_plugin_shell::ShellExt;

    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path
        .parent()
        .ok_or("Failed to get exe directory")?
        .to_string_lossy();
    let pid = std::process::id(); // PID текущего лаунчера

    // Скрипт:
    // 1. Ждет завершения процесса лаунчера (по PID).
    // 2. Распаковывает архив в папку с лаунчером.
    // 3. Запускает лаунчер заново.
    let log_path = std::env::current_dir()
        .unwrap_or_default()
        .join("update.log");
    let script = format!(
        "$logPath = '{}'; \
         try {{ \
             Add-Content -Path $logPath -Value \"$(Get-Date): Starting update script.\"; \
             $pidToWait = {}; \
             $proc = Get-Process -Id $pidToWait -ErrorAction SilentlyContinue; \
             if ($proc) {{ \
                 Add-Content -Path $logPath -Value \"$(Get-Date): Waiting for process $pidToWait to exit...\"; \
                 $proc.WaitForExit(); \
                 Add-Content -Path $logPath -Value \"$(Get-Date): Process exited.\"; \
             }}; \
             Add-Content -Path $logPath -Value \"$(Get-Date): Extracting '{}' to '{}'...\"; \
             tar -xzf '{}' -C '{}'; \
             Add-Content -Path $logPath -Value \"$(Get-Date): Extraction completed.\"; \
             $tempFolder = Split-Path -Path '{}' -Parent; \
             Add-Content -Path $logPath -Value \"$(Get-Date): Cleaning up temp folder contents in $tempFolder...\"; \
             Remove-Item -Path \"$tempFolder\\*\" -Force -Recurse -ErrorAction SilentlyContinue; \
             Add-Content -Path $logPath -Value \"$(Get-Date): Starting process '{}'...\"; \
             Start-Process '{}'; \
             Add-Content -Path $logPath -Value \"$(Get-Date): Process started.\"; \
         }} catch {{ \
             Add-Content -Path $logPath -Value \"$(Get-Date): Error: $($_.Exception.Message)\"; \
         }}",
        log_path.to_string_lossy(),
        pid,
        temp_archive_path,
        exe_dir,
        temp_archive_path,
        exe_dir,
        temp_archive_path,
        exe_path.to_string_lossy(),
        exe_path.to_string_lossy()
    );

    log_update("Spawning update script and exiting...");

    // Запускаем скрипт в отдельном процессе, который не зависит от лаунчера
    app.shell()
        .command("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .spawn()
        .map_err(|e| e.to_string())?;

    app.exit(0);
    Ok(())
}

fn log_update(message: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::time::SystemTime;

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("update.log")
    {
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(file, "[{}] {}", now, message);
    }
}

// ─── Helpers ───────────────────────────────────────────────────────────────────

fn get_game_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(appdata).join(".femboy").join("minecraft")
}

fn get_femboy_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(appdata).join(".femboy")
}

fn get_jre_dir() -> PathBuf {
    get_femboy_dir().join("jre")
}

fn check_sha1(path: &PathBuf, expected: &str) -> bool {
    use sha1::{Digest, Sha1};
    if let Ok(data) = fs::read(path) {
        let hash = Sha1::digest(&data);
        hex::encode(hash) == expected
    } else {
        false
    }
}

async fn download_file_to(
    client: &reqwest::Client,
    url: &str,
    dest: &PathBuf,
    expected_sha1: &str,
    downloaded: Arc<AtomicU64>,
) -> Result<(), String> {
    // Skip if already valid
    if dest.exists() && check_sha1(dest, expected_sha1) {
        downloaded.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    let mut stream = response.bytes_stream();
    let mut file = fs::File::create(dest).map_err(|e| e.to_string())?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
    }

    downloaded.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

async fn download_file_to_optional_sha1(
    client: &reqwest::Client,
    url: &str,
    dest: &PathBuf,
    expected_sha1: Option<&str>,
    downloaded: Arc<AtomicU64>,
) -> Result<(), String> {
    if dest.exists() {
        if let Some(sha1) = expected_sha1 {
            if check_sha1(dest, sha1) {
                downloaded.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
        } else {
            downloaded.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Download failed {}: {}", url, response.status()));
    }

    let mut stream = response.bytes_stream();
    let mut file = fs::File::create(dest).map_err(|e| e.to_string())?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
    }

    downloaded.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

async fn download_version_libraries(
    app: &AppHandle,
    client: &reqwest::Client,
    game_dir: &PathBuf,
    version_id: &str,
    version_json: &VersionJson,
) -> Result<(), String> {
    let libs_dir = game_dir.join("libraries");
    let natives_dir = game_dir.join("natives").join(version_id);
    fs::create_dir_all(&natives_dir).map_err(|e| e.to_string())?;

    let libs: Vec<_> = version_json
        .libraries
        .iter()
        .filter(|lib| should_use_library(lib))
        .collect();

    let total_libs = libs.len() as u64;
    let libs_downloaded = Arc::new(AtomicU64::new(0));

    for lib in &libs {
        let current = libs_downloaded.load(Ordering::Relaxed);
        emit_progress(
            app,
            "libraries",
            current,
            total_libs,
            &format!("Libraries: {}/{}", current, total_libs),
        );

        if let Some(lib_downloads) = &lib.downloads {
            if let Some(artifact) = &lib_downloads.artifact {
                let dest = libs_dir.join(&artifact.path);
                download_file_to(
                    client,
                    &artifact.url,
                    &dest,
                    &artifact.sha1,
                    libs_downloaded.clone(),
                )
                .await?;
            }

            if let Some(natives_map) = &lib.natives {
                if let Some(classifier_key) = natives_map.get("windows") {
                    if let Some(classifiers) = &lib_downloads.classifiers {
                        if let Some(native_artifact) = classifiers.get(classifier_key) {
                            let native_jar_dest = libs_dir.join(&native_artifact.path);
                            download_file_to(
                                client,
                                &native_artifact.url,
                                &native_jar_dest,
                                &native_artifact.sha1,
                                libs_downloaded.clone(),
                            )
                            .await?;

                            extract_natives(&native_jar_dest, &natives_dir, &lib.extract)?;
                        }
                    }
                }
            } else if let Some(artifact) = &lib_downloads.artifact {
                if artifact.path.contains("natives-windows") {
                    let dest = libs_dir.join(&artifact.path);
                    extract_natives(&dest, &natives_dir, &lib.extract)?;
                }
            }
        } else if let (Some(path), Some(base_url)) = (get_library_path(lib), &lib.url) {
            let url = format!("{}{}", base_url.trim_end_matches('/'), format!("/{}", path));
            let dest = libs_dir.join(path);
            download_file_to_optional_sha1(client, &url, &dest, None, libs_downloaded.clone())
                .await?;
        } else {
            libs_downloaded.fetch_add(1, Ordering::Relaxed);
        }
    }

    emit_progress(
        app,
        "libraries",
        total_libs,
        total_libs,
        "Libraries installed",
    );
    Ok(())
}

fn should_use_library(lib: &Library) -> bool {
    if let Some(rules) = &lib.rules {
        let mut allowed = false;
        for rule in rules {
            let os_match = match &rule.os {
                None => true,
                Some(os_rule) => match &os_rule.name {
                    None => true,
                    Some(name) => name == "windows",
                },
            };
            if os_match {
                allowed = rule.action == "allow";
            }
        }
        allowed
    } else {
        true
    }
}

fn generate_offline_uuid(username: &str) -> String {
    let namespace =
        uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap_or(uuid::Uuid::nil());
    let player_uuid =
        uuid::Uuid::new_v5(&namespace, format!("OfflinePlayer:{}", username).as_bytes());
    player_uuid.to_string()
}

#[tauri::command]
fn get_game_dir_path() -> String {
    get_game_dir().to_string_lossy().to_string()
}

fn get_playtime_file_path() -> PathBuf {
    get_femboy_dir().join("playtime.json")
}

#[tauri::command]
fn get_playtime_stats() -> Result<PlaytimeStats, String> {
    let path = get_playtime_file_path();
    if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        if let Ok(stats) = serde_json::from_str::<PlaytimeStats>(&content) {
            return Ok(stats);
        }
    }
    Ok(PlaytimeStats::default())
}

fn append_playtime_session(session: PlaySession) {
    let mut stats = get_playtime_stats().unwrap_or_default();
    stats.sessions.push(session);
    let path = get_playtime_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string(&stats) {
        let _ = fs::write(&path, content);
    }
}

#[tauri::command]
fn load_accounts() -> Result<Vec<Account>, String> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(appdata).join(".femboy").join("sessions");
    if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;

        // Попытка спарсить как новый формат
        if let Ok(accounts) = serde_json::from_str::<Vec<Account>>(&content) {
            return Ok(accounts);
        }

        // Если это старый формат (массив строк), мигрируем на Offline аккаунты
        if let Ok(legacy_accounts) = serde_json::from_str::<Vec<String>>(&content) {
            let accounts: Vec<Account> = legacy_accounts
                .into_iter()
                .map(|username| Account::Offline { username })
                .collect();
            // Сразу сохраним в новом формате
            let _ = save_accounts(accounts.clone());
            return Ok(accounts);
        }

        Ok(Vec::new())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn save_accounts(accounts: Vec<Account>) -> Result<(), String> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(appdata).join(".femboy").join("sessions");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string(&accounts).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn login_ely(app: AppHandle) -> Result<Account, String> {
    use tauri_plugin_shell::ShellExt;

    let client_id = "femboy-launcher";
    let client_secret = "GmHWtUwSIGp9lmqEgGNKZ_fWQil77F4kDcgRAXGRHFM0CW7nZxcq0wlBc8zf84Js";
    let redirect_uri = "http://localhost:3456/";

    let state = format!("{:x}", rand::random::<u64>());

    let mut auth_url =
        reqwest::Url::parse("https://account.ely.by/oauth2/v1").map_err(|e| e.to_string())?;
    {
        let mut query = auth_url.query_pairs_mut();
        query.append_pair("client_id", client_id);
        query.append_pair("redirect_uri", redirect_uri);
        query.append_pair("response_type", "code");
        query.append_pair("prompt", "select_account");
        query.append_pair("state", &state);
        query.append_pair("scope", "account_info");
    }
    let auth_url = auth_url.to_string();

    #[allow(deprecated)]
    let _ = app.shell().open(auth_url, None).map_err(|e| e.to_string());

    // Запускаем TCP-сервер
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3456")
        .await
        .map_err(|e| e.to_string())?;
    let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;

    let mut buffer = [0; 4096];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buffer)
        .await
        .map_err(|e| e.to_string())?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    let mut code = String::new();
    let mut returned_state = String::new();
    let mut error_name = String::new();

    if let Some(line) = request.lines().next() {
        if line.starts_with("GET ") {
            if let Some(full_path) = line.split_whitespace().nth(1) {
                // Парсим URL через reqwest::Url для корректного декодирования параметров
                let base = "http://localhost:3456";
                let url = reqwest::Url::parse(&format!("{}{}", base, full_path))
                    .unwrap_or_else(|_| reqwest::Url::parse(base).unwrap());
                for (key, value) in url.query_pairs() {
                    match key.as_ref() {
                        "code" => code = value.into_owned(),
                        "state" => returned_state = value.into_owned(),
                        "error" => error_name = value.into_owned(),
                        _ => {}
                    }
                }
            }
        }
    }

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<html><head><title>Ely.by Auth</title><style>body { font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; background: #1a1a1a; color: white; }</style></head><body><h2>Авторизация успешна! Можете закрыть это окно.</h2><script>setTimeout(()=>window.close(), 2000)</script></body></html>";
    let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;

    if !error_name.is_empty() {
        return Err(format!("Ошибка Ely.by: {}", error_name));
    }

    if returned_state != state {
        return Err("Ошибка безопасности: state mismatch".to_string());
    }

    if code.is_empty() {
        return Err("Код авторизации не получен".to_string());
    }

    let client = reqwest::Client::new();
    let token_res = client
        .post("https://account.ely.by/api/oauth2/v1/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
            ("code", &code),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !token_res.status().is_success() {
        let err_body = token_res.text().await.unwrap_or_default();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&err_body) {
            if let Some(desc) = json["error_description"].as_str() {
                return Err(format!("Ошибка получения токена: {}", desc));
            }
            if let Some(err_name) = json["error"].as_str() {
                return Err(format!("Ошибка получения токена: {}", err_name));
            }
        }
        return Err(format!("Ошибка получения токена: {}", err_body));
    }

    let token_data: serde_json::Value = token_res.json().await.map_err(|e| e.to_string())?;
    let access_token = token_data["access_token"]
        .as_str()
        .ok_or("Нет access_token")?
        .to_string();

    let info_res = client
        .get("https://account.ely.by/api/account/v1/info")
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !info_res.status().is_success() {
        return Err("Ошибка получения профиля".to_string());
    }

    let info_data: serde_json::Value = info_res.json().await.map_err(|e| e.to_string())?;
    let username = info_data["username"]
        .as_str()
        .ok_or("Нет имени")?
        .to_string();
    let uuid = info_data["uuid"].as_str().ok_or("Нет uuid")?.to_string();

    Ok(Account::Ely {
        username,
        uuid,
        access_token,
    })
}

#[tauri::command]
async fn login_microsoft(app: AppHandle) -> Result<Account, String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use sha2::{Digest, Sha256};
    use tauri_plugin_shell::ShellExt;

    let client_id = "8f9d9c8f-2dfb-40ff-a498-b6c695bbb44a"; // Default testing ID if needed, user should use their own
    let redirect_uri = "http://localhost:3457/";
    let microsoft_oauth_base = "https://login.microsoftonline.com/consumers/oauth2/v2.0";

    let state = format!("{:x}", rand::random::<u64>());

    // Generate PKCE code verifier and challenge
    let mut verifier_bytes = [0u8; 32];
    for b in &mut verifier_bytes {
        *b = rand::random::<u8>();
    }
    let code_verifier = URL_SAFE_NO_PAD.encode(&verifier_bytes);

    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    let mut auth_url = reqwest::Url::parse(&format!("{}/authorize", microsoft_oauth_base))
        .map_err(|e| e.to_string())?;
    {
        let mut query = auth_url.query_pairs_mut();
        query.append_pair("client_id", client_id);
        query.append_pair("response_type", "code");
        query.append_pair("redirect_uri", redirect_uri);
        query.append_pair("scope", "openid profile XboxLive.signin offline_access");
        query.append_pair("state", &state);
        query.append_pair("code_challenge", &code_challenge);
        query.append_pair("code_challenge_method", "S256");
        query.append_pair("prompt", "select_account");
    }

    #[allow(deprecated)]
    let _ = app
        .shell()
        .open(auth_url.to_string(), None)
        .map_err(|e| e.to_string());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3457")
        .await
        .map_err(|e| e.to_string())?;
    let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;

    let mut buffer = [0; 4096];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buffer)
        .await
        .map_err(|e| e.to_string())?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    let mut code = String::new();
    let mut returned_state = String::new();
    let mut error_name = String::new();
    let mut error_description = String::new();

    if let Some(line) = request.lines().next() {
        if line.starts_with("GET ") {
            if let Some(full_path) = line.split_whitespace().nth(1) {
                let base = "http://localhost:3457";
                let url = reqwest::Url::parse(&format!("{}{}", base, full_path))
                    .unwrap_or_else(|_| reqwest::Url::parse(base).unwrap());
                for (key, value) in url.query_pairs() {
                    match key.as_ref() {
                        "code" => code = value.into_owned(),
                        "state" => returned_state = value.into_owned(),
                        "error" => error_name = value.into_owned(),
                        "error_description" => error_description = value.into_owned(),
                        _ => {}
                    }
                }
            }
        }
    }

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<html><head><title>Microsoft Auth</title><style>body { font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; background: #1a1a1a; color: white; }</style></head><body><h2>Авторизация успешна! Можете закрыть это окно.</h2><script>setTimeout(()=>window.close(), 2000)</script></body></html>";
    let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;

    if !error_name.is_empty() {
        if error_description.is_empty() {
            return Err(format!("Ошибка Microsoft OAuth: {}", error_name));
        }

        return Err(format!(
            "Ошибка Microsoft OAuth: {} ({})",
            error_name, error_description
        ));
    }
    if returned_state != state {
        return Err("Ошибка безопасности: state mismatch".to_string());
    }
    if code.is_empty() {
        return Err("Код авторизации не получен".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| format!("Ошибка сборки HTTP-клиента: {}", e))?;

    // 1. Get MS Token
    let token_res = client
        .post(format!("{}/token", microsoft_oauth_base))
        .form(&[
            ("client_id", client_id),
            ("code", &code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
            ("code_verifier", &code_verifier),
        ])
        .send()
        .await
        .map_err(|e| format!("MS Token req error: {}", e))?;

    if !token_res.status().is_success() {
        let text = token_res.text().await.unwrap_or_default();
        return Err(format!("MS Token error: {}", text));
    }

    let ms_token_data: serde_json::Value = token_res
        .json()
        .await
        .map_err(|e| format!("MS Token json error: {}", e))?;
    let ms_access_token = ms_token_data["access_token"]
        .as_str()
        .ok_or("No MS access_token")?;

    // 2. Auth with Xbox Live
    let xbl_req_body = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={}", ms_access_token)
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let xbl_res = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&xbl_req_body)
        .send()
        .await
        .map_err(|e| format!("XBL req error: {}", e))?;

    if !xbl_res.status().is_success() {
        let text = xbl_res.text().await.unwrap_or_default();
        return Err(format!("XBL error: {}", text));
    }

    let xbl_data: serde_json::Value = xbl_res
        .json()
        .await
        .map_err(|e| format!("XBL json error: {}", e))?;
    let xbl_token = xbl_data["Token"].as_str().ok_or("No XBL Token")?;
    let _xbl_uhs = xbl_data["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or("No XBL uhs")?;

    // 3. Get XSTS token
    let xsts_req_body = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let xsts_res = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&xsts_req_body)
        .send()
        .await
        .map_err(|e| format!("XSTS req error: {}", e))?;

    if !xsts_res.status().is_success() {
        let text = xsts_res.text().await.unwrap_or_default();
        return Err(format!("XSTS error: {}", text));
    }

    let xsts_data: serde_json::Value = xsts_res
        .json()
        .await
        .map_err(|e| format!("XSTS json error: {}", e))?;
    let xsts_token = xsts_data["Token"].as_str().ok_or("No XSTS Token")?;
    let xsts_uhs = xsts_data["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or("No XSTS uhs")?;

    // 4. Authenticate with Minecraft
    let mc_req_body = serde_json::json!({
        "identityToken": format!("XBL3.0 x={};{}", xsts_uhs, xsts_token)
    });

    let mc_res = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&mc_req_body)
        .send()
        .await
        .map_err(|e| format!("MC req error: {}", e))?;

    if !mc_res.status().is_success() {
        let text = mc_res.text().await.unwrap_or_default();
        return Err(format!("MC error: {}", text));
    }

    let mc_data: serde_json::Value = mc_res
        .json()
        .await
        .map_err(|e| format!("MC json error: {}", e))?;
    let mc_access_token = mc_data["access_token"]
        .as_str()
        .ok_or("No MC access_token")?;

    // 5. Get Minecraft Profile
    let profile_res = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(mc_access_token)
        .send()
        .await
        .map_err(|e| format!("MC profile req error: {}", e))?;

    if !profile_res.status().is_success() {
        let text = profile_res.text().await.unwrap_or_default();
        return Err(format!(
            "MC profile error (does the account own Minecraft?): {}",
            text
        ));
    }

    let profile_data: serde_json::Value = profile_res
        .json()
        .await
        .map_err(|e| format!("MC profile json error: {}", e))?;
    let mc_name = profile_data["name"].as_str().ok_or("No MC name")?;
    let mc_uuid = profile_data["id"].as_str().ok_or("No MC id")?;

    Ok(Account::Microsoft {
        username: mc_name.to_string(),
        uuid: mc_uuid.to_string(),
        access_token: mc_access_token.to_string(),
    })
}

#[tauri::command]
fn load_settings() -> Result<serde_json::Value, String> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(appdata).join(".femboy").join("settings.json");
    if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let val: serde_json::Value =
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
        Ok(val)
    } else {
        Ok(serde_json::json!({}))
    }
}

#[tauri::command]
fn save_settings(settings: serde_json::Value) -> Result<(), String> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(appdata).join(".femboy").join("settings.json");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

/// Парсит Forge display ID вида "Forge 1.12.2" или "ForgeOptifine 1.12.2" и возвращает (forge_dir_id, mc_ver, forge_build, is_optifine).
/// forge_dir_id = "{mc_ver}-forge-{forge_build}" — имя папки/файла версии.
fn parse_forge_display(id: &str) -> Option<(String, String, String, bool)> {
    let is_optifine = id.starts_with("ForgeOptifine ");
    let mc_ver = if is_optifine {
        id.strip_prefix("ForgeOptifine ")?.to_string()
    } else {
        id.strip_prefix("Forge ")?.to_string()
    };
    // forge_build хранится в id после разделителя " | "
    // Формат display: "Forge {mc_ver} | {forge_build}"
    if let Some(idx) = mc_ver.find(" | ") {
        let mc = mc_ver[..idx].to_string();
        let forge_build = mc_ver[idx + 3..].to_string();
        let dir_id = format!("{}-forge-{}", mc, forge_build);
        return Some((dir_id, mc, forge_build, is_optifine));
    }
    None
}

/// Парсит Fabric display ID вида "Fabric 1.21.1 | 0.15.11"
fn parse_fabric_display(id: &str) -> Option<(String, String, String)> {
    let mc_ver = id.strip_prefix("Fabric ")?.to_string();
    if let Some(idx) = mc_ver.find(" | ") {
        let mc = mc_ver[..idx].to_string();
        let loader_ver = mc_ver[idx + 3..].to_string();
        let dir_id = format!("{}-fabric-{}", mc, loader_ver);
        return Some((dir_id, mc, loader_ver));
    }
    None
}

/// Parses NeoForge display ID like "NeoForge 1.21.1 | 21.1.200".
fn parse_neoforge_display(id: &str) -> Option<(String, String, String)> {
    let mc_ver = id
        .strip_prefix("NeoForge ")
        .or_else(|| id.strip_prefix("Neoforge "))?
        .to_string();
    if let Some(idx) = mc_ver.find(" | ") {
        let mc = mc_ver[..idx].to_string();
        let loader_ver = mc_ver[idx + 3..].to_string();
        let dir_id = format!("neoforge-{}", loader_ver);
        return Some((dir_id, mc, loader_ver));
    }
    None
}

fn required_java_major(mc_version: &str) -> u32 {
    let parts: Vec<u32> = mc_version
        .split('.')
        .take(3)
        .filter_map(|part| part.parse::<u32>().ok())
        .collect();

    let major = parts.first().copied().unwrap_or(1);
    let minor = parts.get(1).copied().unwrap_or(0);
    let patch = parts.get(2).copied().unwrap_or(0);

    if major >= 26 || major > 1 || minor > 21 || (minor == 21 && patch > 11) {
        25
    } else if minor > 20 || (minor == 20 && patch >= 5) {
        21
    } else if minor >= 17 {
        17
    } else {
        8
    }
}

fn java_major(java_cmd: &str) -> Option<u32> {
    let output = std::process::Command::new(java_cmd)
        .arg("-version")
        .output()
        .ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let version_marker = "version \"";
    let start = text.find(version_marker)? + version_marker.len();
    let version = text[start..].split('"').next()?;

    if let Some(rest) = version.strip_prefix("1.") {
        rest.split('.').next()?.parse().ok()
    } else {
        version.split('.').next()?.parse().ok()
    }
}

fn normalize_java_preference(preference: Option<&str>) -> Option<u32> {
    match preference.unwrap_or("recommended") {
        "8" => Some(8),
        "17" => Some(17),
        "21" => Some(21),
        "25" => Some(25),
        _ => None,
    }
}

fn saved_java_preference() -> Option<String> {
    load_settings()
        .ok()
        .and_then(|settings| settings.get("javaPreference")?.as_str().map(str::to_string))
}

fn local_java_command(major: u32) -> PathBuf {
    get_jre_dir()
        .join(format!("java-{}", major))
        .join("bin")
        .join("java.exe")
}

async fn ensure_jre(
    app: &AppHandle,
    client: &reqwest::Client,
    major: u32,
) -> Result<PathBuf, String> {
    let java_cmd = local_java_command(major);
    if java_cmd.exists() && java_major(&java_cmd.to_string_lossy()) == Some(major) {
        emit_progress(
            app,
            "java",
            1,
            1,
            &format!("Java {} already installed", major),
        );
        return Ok(java_cmd);
    }

    let jre_dir = get_jre_dir();
    let install_dir = jre_dir.join(format!("java-{}", major));
    fs::create_dir_all(&jre_dir).map_err(|e| e.to_string())?;

    emit_progress(app, "java", 0, 1, &format!("Downloading Java {}...", major));

    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{}/ga/windows/x64/jre/hotspot/normal/eclipse?project=jdk",
        major
    );
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to download Java {}: {}",
            major,
            response.status()
        ));
    }

    let archive_path = jre_dir.join(format!("java-{}.zip", major));
    let mut file = fs::File::create(&archive_path).map_err(|e| e.to_string())?;
    let total = response.content_length().unwrap_or(0);
    let mut downloaded = 0_u64;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        emit_progress(
            app,
            "java",
            downloaded,
            total.max(downloaded),
            &format!("Downloading Java {}...", major),
        );
    }
    drop(file);

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&install_dir).map_err(|e| e.to_string())?;

    emit_progress(app, "java", 0, 1, &format!("Installing Java {}...", major));
    extract_jre_archive(&archive_path, &install_dir)?;
    let _ = fs::remove_file(&archive_path);

    if java_cmd.exists() && java_major(&java_cmd.to_string_lossy()) == Some(major) {
        emit_progress(app, "java", 1, 1, &format!("Java {} installed", major));
        Ok(java_cmd)
    } else {
        Err(format!("Java {} installation is incomplete", major))
    }
}

fn extract_jre_archive(archive_path: &PathBuf, install_dir: &PathBuf) -> Result<(), String> {
    let file = fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut root_prefix: Option<String> = None;

    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().replace('\\', "/");
        if let Some(first) = name.split('/').find(|part| !part.is_empty()) {
            root_prefix = Some(format!("{}/", first));
            break;
        }
    }

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let raw_name = entry.name().replace('\\', "/");
        let relative = root_prefix
            .as_ref()
            .and_then(|prefix| raw_name.strip_prefix(prefix))
            .unwrap_or(&raw_name);
        if relative.is_empty() || relative.ends_with('/') {
            continue;
        }
        if std::path::Path::new(relative)
            .components()
            .any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            return Err("Unsafe path in Java archive".to_string());
        }

        let out_path = install_dir.join(relative);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = fs::File::create(&out_path).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn select_java_command(
    app: &AppHandle,
    client: &reqwest::Client,
    mc_version: &str,
    preference: Option<&str>,
) -> Result<String, String> {
    let major =
        normalize_java_preference(preference).unwrap_or_else(|| required_java_major(mc_version));
    let java_cmd = ensure_jre(app, client, major).await?;
    Ok(java_cmd.to_string_lossy().into_owned())
}

fn get_library_path(lib: &Library) -> Option<String> {
    if let Some(downloads) = &lib.downloads {
        if let Some(artifact) = &downloads.artifact {
            return Some(artifact.path.clone());
        }
    }
    let parts: Vec<&str> = lib.name.split(':').collect();
    if parts.len() >= 3 {
        let group = parts[0].replace('.', "/");
        let artifact = parts[1];
        let version = parts[2];
        let classifier = if parts.len() > 3 {
            format!("-{}", parts[3])
        } else {
            "".to_string()
        };
        return Some(format!(
            "{}/{}/{}/{}-{}{}.jar",
            group, artifact, version, artifact, version, classifier
        ));
    }
    None
}

#[derive(Deserialize)]
struct FabricGameVersion {
    version: String,
    stable: bool,
}

#[derive(Deserialize)]
struct FabricLoaderVersion {
    version: String,
    stable: bool,
}

#[derive(Deserialize)]
struct FabricMeta {
    game: Vec<FabricGameVersion>,
    loader: Vec<FabricLoaderVersion>,
}

fn extract_xml_tag_values(xml: &str, tag: &str) -> Vec<String> {
    let start = format!("<{}>", tag);
    let end = format!("</{}>", tag);
    let mut values = Vec::new();
    let mut rest = xml;

    while let Some(start_idx) = rest.find(&start) {
        let value_start = start_idx + start.len();
        if let Some(end_idx) = rest[value_start..].find(&end) {
            values.push(rest[value_start..value_start + end_idx].trim().to_string());
            rest = &rest[value_start + end_idx + end.len()..];
        } else {
            break;
        }
    }

    values
}

fn minecraft_version_from_neoforge_version(version: &str) -> Option<String> {
    let mut parts = version.split('.');
    let minor = parts.next()?.parse::<u32>().ok()?;
    let patch = parts.next()?.parse::<u32>().ok()?;

    Some(if patch == 0 {
        format!("1.{}", minor)
    } else {
        format!("1.{}.{}", minor, patch)
    })
}

async fn fetch_neoforge_meta(client: &reqwest::Client) -> Option<HashMap<String, String>> {
    let xml = client
        .get("https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml")
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;

    let mut latest_by_mc: HashMap<String, String> = HashMap::new();
    for version in extract_xml_tag_values(&xml, "version") {
        if let Some(mc_version) = minecraft_version_from_neoforge_version(&version) {
            latest_by_mc.insert(mc_version, version);
        }
    }

    Some(latest_by_mc)
}

fn get_local_versions() -> Vec<VersionInfo> {
    let mut result = Vec::new();
    let game_dir = get_game_dir();
    let versions_dir = game_dir.join("versions");

    if let Ok(entries) = fs::read_dir(versions_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    let json_path = entry.path().join(format!("{}.json", dir_name));
                    if json_path.exists() {
                        if let Some((mc, forge)) = dir_name.split_once("-forge-") {
                            result.push(VersionInfo {
                                id: format!("Forge {} | {}", mc, forge),
                                installed: true,
                            });
                            result.push(VersionInfo {
                                id: format!("ForgeOptifine {} | {}", mc, forge),
                                installed: true,
                            });
                        } else if let Some((mc, fabric)) = dir_name.split_once("-fabric-") {
                            result.push(VersionInfo {
                                id: format!("Fabric {} | {}", mc, fabric),
                                installed: true,
                            });
                        } else if let Some(loader) = dir_name.strip_prefix("neoforge-") {
                            // Try to find MC version from the JSON
                            let mc_ver = if let Ok(content) = fs::read_to_string(&json_path) {
                                if let Ok(json) =
                                    serde_json::from_str::<serde_json::Value>(&content)
                                {
                                    json["install"]["minecraft"]
                                        .as_str()
                                        .map(|s| s.to_string())
                                        .or_else(|| {
                                            // Fallback for some NeoForge versions
                                            json["arguments"]["game"]
                                                .as_array()
                                                .and_then(|args| {
                                                    args.iter()
                                                        .position(|a| {
                                                            a.as_str() == Some("--version")
                                                        })
                                                        .and_then(|i| {
                                                            args.get(i + 1).and_then(|v| v.as_str())
                                                        })
                                                })
                                                .map(|s| s.to_string())
                                        })
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            let display_id = match mc_ver {
                                Some(mc) => format!("NeoForge {} | {}", mc, loader),
                                None => format!("NeoForge | {}", loader),
                            };
                            result.push(VersionInfo {
                                id: display_id,
                                installed: true,
                            });
                        } else {
                            // Vanilla
                            result.push(VersionInfo {
                                id: dir_name.clone(),
                                installed: true,
                            });
                        }
                    }
                }
            }
        }
    }

    // Sort versions (newest first is better, but local order is fine for fallback)
    result.sort_by(|a, b| b.id.cmp(&a.id));
    result
}

#[tauri::command]
async fn get_versions() -> Result<Vec<VersionInfo>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    // Fetch Mojang version manifest
    let manifest_res = client
        .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .send()
        .await;

    let manifest: VersionManifest = match manifest_res {
        Ok(res) => match res.json().await {
            Ok(m) => m,
            Err(_) => return Ok(get_local_versions()),
        },
        Err(_) => return Ok(get_local_versions()),
    };

    // Fetch Forge maven metadata (mc_version -> [forge builds])
    let forge_meta: HashMap<String, Vec<String>> = match client
        .get("https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json")
        .send()
        .await
    {
        Ok(res) => res.json().await.unwrap_or_default(),
        Err(_) => HashMap::new(),
    };

    // Fetch Fabric metadata
    let fabric_meta: Option<FabricMeta> = match client
        .get("https://meta.fabricmc.net/v2/versions")
        .send()
        .await
    {
        Ok(res) => res.json().await.ok(),
        Err(_) => None,
    };

    let latest_fabric_loader = fabric_meta.as_ref().and_then(|m| {
        m.loader
            .iter()
            .find(|l| l.stable)
            .map(|l| l.version.clone())
    });
    let fabric_games: Vec<String> = fabric_meta
        .as_ref()
        .map(|m| m.game.iter().map(|g| g.version.clone()).collect())
        .unwrap_or_default();
    let neoforge_meta = fetch_neoforge_meta(&client).await.unwrap_or_default();

    let game_dir = get_game_dir();

    // Only release versions >= 1.6, in manifest order (newest first)
    let release_versions: Vec<VersionEntry> = manifest
        .versions
        .into_iter()
        .filter(|v| v.version_type == "release" && is_version_gte_1_6(&v.id))
        .collect();

    let mut result: Vec<VersionInfo> = Vec::new();

    for v in &release_versions {
        // Check if Forge supports this MC version
        if let Some(forge_builds) = forge_meta.get(&v.id) {
            if let Some(last_build) = forge_builds.last() {
                // Build ID from maven entry: "1.12.2-14.23.5.2864" -> forge_build = "14.23.5.2864"
                let forge_build = last_build
                    .strip_prefix(&format!("{}-", v.id))
                    .unwrap_or(last_build);
                let forge_dir_id = format!("{}-forge-{}", v.id, forge_build);
                let display_id = format!("Forge {} | {}", v.id, forge_build);
                let optifine_id = format!("ForgeOptifine {} | {}", v.id, forge_build);
                let forge_json_path = game_dir
                    .join("versions")
                    .join(&forge_dir_id)
                    .join(format!("{}.json", forge_dir_id));
                let installed = forge_json_path.exists();
                result.push(VersionInfo {
                    id: display_id,
                    installed,
                });
                result.push(VersionInfo {
                    id: optifine_id,
                    installed,
                });
            }
        }

        // Check if Fabric supports this MC version
        if let Some(loader_ver) = &latest_fabric_loader {
            if fabric_games.contains(&v.id) {
                let fabric_dir_id = format!("{}-fabric-{}", v.id, loader_ver);
                let display_id = format!("Fabric {} | {}", v.id, loader_ver);
                let fabric_json_path = game_dir
                    .join("versions")
                    .join(&fabric_dir_id)
                    .join(format!("{}.json", fabric_dir_id));
                result.push(VersionInfo {
                    id: display_id,
                    installed: fabric_json_path.exists(),
                });
            }
        }

        // Check if NeoForge supports this MC version
        if let Some(loader_ver) = neoforge_meta.get(&v.id) {
            let neoforge_dir_id = format!("neoforge-{}", loader_ver);
            let display_id = format!("NeoForge {} | {}", v.id, loader_ver);
            let neoforge_json_path = game_dir
                .join("versions")
                .join(&neoforge_dir_id)
                .join(format!("{}.json", neoforge_dir_id));
            result.push(VersionInfo {
                id: display_id,
                installed: neoforge_json_path.exists(),
            });
        }

        // Then vanilla version
        let vanilla_json_path = game_dir
            .join("versions")
            .join(&v.id)
            .join(format!("{}.json", &v.id));
        result.push(VersionInfo {
            id: v.id.clone(),
            installed: vanilla_json_path.exists(),
        });
    }

    Ok(result)
}

fn is_version_gte_1_6(id: &str) -> bool {
    // Parse major.minor from version like "1.21.4", "1.6.4", "1.7", etc.
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    let major: u32 = parts[0].parse().unwrap_or(0);
    let minor: u32 = parts[1].parse().unwrap_or(0);
    if major > 1 {
        return true;
    }
    if major == 1 && minor >= 6 {
        return true;
    }
    false
}

#[tauri::command]
async fn is_version_installed(version_id: String) -> bool {
    let real_id = if let Some((forge_dir_id, _, _, _)) = parse_forge_display(&version_id) {
        forge_dir_id
    } else if let Some((fabric_dir_id, _, _)) = parse_fabric_display(&version_id) {
        fabric_dir_id
    } else if let Some((neoforge_dir_id, _, _)) = parse_neoforge_display(&version_id) {
        neoforge_dir_id
    } else {
        version_id.clone()
    };
    let json = get_game_dir()
        .join("versions")
        .join(&real_id)
        .join(format!("{}.json", real_id));
    json.exists()
}

#[tauri::command]
async fn install_version(app: AppHandle, version_id: String) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let game_dir = get_game_dir();
    fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;

    if let Some((forge_dir_id, mc_ver, forge_ver, _)) = parse_forge_display(&version_id) {
        emit_progress(
            &app,
            "manifest",
            0,
            1,
            format!("Установка базовой версии {}...", mc_ver).as_str(),
        );

        // 1. Install vanilla version first so we have the client jar, libraries, and natives.
        install_vanilla(&app, &client, &game_dir, &mc_ver).await?;

        // 2. Install Forge
        emit_progress(&app, "manifest", 0, 1, "Скачивание установщика Forge...");

        // Forge installer requires launcher_profiles.json to exist
        let profiles_path = game_dir.join("launcher_profiles.json");
        if !profiles_path.exists() {
            let minimal_profile = serde_json::json!({
                "profiles": {},
                "selectedProfile": null,
                "clientToken": "femboy-launcher",
                "authenticationDatabase": {}
            });
            fs::write(&profiles_path, minimal_profile.to_string())
                .map_err(|e| format!("Не удалось создать launcher_profiles.json: {}", e))?;
        }

        let installer_url = format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{mc_ver}-{forge_ver}/forge-{mc_ver}-{forge_ver}-installer.jar"
        );
        let installer_path = game_dir.join(format!("forge-installer-{}-{}.jar", mc_ver, forge_ver));

        let response = client
            .get(&installer_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            // Fallback to BMCLAPI mirror
            let mirror_url = format!(
                "https://bmclapi2.bangbang93.com/forge/download?mcversion={}&version={}&category=installer&format=jar",
                mc_ver, forge_ver
            );
            let response2 = client
                .get(&mirror_url)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let bytes = response2.bytes().await.map_err(|e| e.to_string())?;
            fs::write(&installer_path, &bytes).map_err(|e| e.to_string())?;
        } else {
            let bytes = response.bytes().await.map_err(|e| e.to_string())?;
            fs::write(&installer_path, &bytes).map_err(|e| e.to_string())?;
        }

        emit_progress(
            &app,
            "client",
            0,
            1,
            "Установка Forge (это может занять время)...",
        );

        let java_preference = saved_java_preference();
        let java_command =
            select_java_command(&app, &client, &mc_ver, java_preference.as_deref()).await?;
        let status = std::process::Command::new(&java_command)
            .arg("-jar")
            .arg(&installer_path)
            .arg("--installClient")
            .arg(&game_dir)
            .status()
            .map_err(|e| format!("Ошибка запуска Java: {}", e))?;

        if !status.success() {
            return Err(format!(
                "Ошибка установки Forge {}. Убедитесь что установлена Java 8 (для 1.12.2) или Java 17+ (для новых версий).",
                forge_dir_id
            ));
        }

        let _ = std::fs::remove_file(&installer_path);
        let _ = std::fs::remove_file(installer_path.with_extension("jar.log"));

        emit_progress(&app, "done", 1, 1, "Установка завершена!");
        return Ok(());
    } else if let Some((neoforge_dir_id, mc_ver, loader_ver)) = parse_neoforge_display(&version_id)
    {
        emit_progress(
            &app,
            "manifest",
            0,
            1,
            format!("Установка базовой версии {}...", mc_ver).as_str(),
        );

        install_vanilla(&app, &client, &game_dir, &mc_ver).await?;

        emit_progress(&app, "manifest", 0, 1, "Скачивание установщика NeoForge...");

        let profiles_path = game_dir.join("launcher_profiles.json");
        if !profiles_path.exists() {
            let minimal_profile = serde_json::json!({
                "profiles": {},
                "selectedProfile": null,
                "clientToken": "femboy-launcher",
                "authenticationDatabase": {}
            });
            fs::write(&profiles_path, minimal_profile.to_string())
                .map_err(|e| format!("Не удалось создать launcher_profiles.json: {}", e))?;
        }

        let installer_url = format!(
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/{loader_ver}/neoforge-{loader_ver}-installer.jar"
        );
        let installer_path = game_dir.join(format!("neoforge-installer-{}.jar", loader_ver));

        let response = client
            .get(&installer_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "Ошибка скачивания установщика NeoForge: {}",
                response.status()
            ));
        }
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        fs::write(&installer_path, &bytes).map_err(|e| e.to_string())?;

        emit_progress(
            &app,
            "client",
            0,
            1,
            "Установка NeoForge (это может занять время)...",
        );

        let java_preference = saved_java_preference();
        let java_command =
            select_java_command(&app, &client, &mc_ver, java_preference.as_deref()).await?;
        let status = std::process::Command::new(&java_command)
            .arg("-jar")
            .arg(&installer_path)
            .arg("--installClient")
            .arg(&game_dir)
            .status()
            .map_err(|e| format!("Ошибка запуска Java: {}", e))?;

        if !status.success() {
            return Err(format!("Ошибка установки NeoForge {}.", neoforge_dir_id));
        }

        let _ = std::fs::remove_file(&installer_path);

        emit_progress(&app, "done", 1, 1, "Установка завершена!");
        return Ok(());
    } else if let Some((fabric_dir_id, mc_ver, loader_ver)) = parse_fabric_display(&version_id) {
        emit_progress(
            &app,
            "manifest",
            0,
            1,
            format!("Установка базовой версии {}...", mc_ver).as_str(),
        );

        install_vanilla(&app, &client, &game_dir, &mc_ver).await?;

        emit_progress(&app, "client", 0, 1, "Скачивание профиля Fabric...");

        let profile_url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
            mc_ver, loader_ver
        );
        let profile_path = game_dir
            .join("versions")
            .join(&fabric_dir_id)
            .join(format!("{}.json", fabric_dir_id));
        fs::create_dir_all(profile_path.parent().unwrap()).map_err(|e| e.to_string())?;

        let response = client
            .get(&profile_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "Ошибка получения профиля Fabric: {}",
                response.status()
            ));
        }
        let profile_bytes = response.bytes().await.map_err(|e| e.to_string())?;
        fs::write(&profile_path, &profile_bytes).map_err(|e| e.to_string())?;

        let fabric_profile: VersionJson =
            serde_json::from_slice(&profile_bytes).map_err(|e| e.to_string())?;
        download_version_libraries(&app, &client, &game_dir, &fabric_dir_id, &fabric_profile)
            .await?;

        emit_progress(&app, "done", 1, 1, "Установка завершена!");
        return Ok(());
    }

    // Install standard vanilla version
    install_vanilla(&app, &client, &game_dir, &version_id).await?;

    emit_progress(&app, "done", 1, 1, "Установка завершена!");
    Ok(())
}

async fn install_vanilla(
    app: &AppHandle,
    client: &reqwest::Client,
    game_dir: &PathBuf,
    version_id: &str,
) -> Result<(), String> {
    // ── 1. Fetch version manifest ───────────────────────────────────────────────
    emit_progress(app, "manifest", 0, 1, "Получение информации о версии...");

    let manifest: VersionManifest = client
        .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let version_entry = manifest
        .versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| format!("Версия {} не найдена", version_id))?
        .clone();

    // ── 2. Fetch version JSON ───────────────────────────────────────────────────
    let version_dir = game_dir.join("versions").join(version_id);
    fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;

    let version_json_path = version_dir.join(format!("{}.json", version_id));
    let version_json_text = client
        .get(&version_entry.url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    fs::write(&version_json_path, &version_json_text).map_err(|e| e.to_string())?;
    let version_json: VersionJson =
        serde_json::from_str(&version_json_text).map_err(|e| e.to_string())?;

    // ── 3. Download client.jar ─────────────────────────────────────────────────
    emit_progress(app, "client", 0, 1, "Скачивание клиента Minecraft...");
    let client_jar_path = version_dir.join(format!("{}.jar", version_id));
    let downloaded_counter = Arc::new(AtomicU64::new(0));

    if let Some(downloads) = &version_json.downloads {
        download_file_to(
            client,
            &downloads.client.url,
            &client_jar_path,
            &downloads.client.sha1,
            downloaded_counter.clone(),
        )
        .await?;
    }
    emit_progress(app, "client", 1, 1, "Клиент Minecraft скачан");

    // ── 4. Download libraries ──────────────────────────────────────────────────
    let libs_dir = game_dir.join("libraries");
    let natives_dir = game_dir.join("natives").join(version_id);
    fs::create_dir_all(&natives_dir).map_err(|e| e.to_string())?;

    let libs: Vec<_> = version_json
        .libraries
        .iter()
        .filter(|lib| should_use_library(lib))
        .collect();

    let total_libs = libs.len() as u64;
    let libs_downloaded = Arc::new(AtomicU64::new(0));

    for lib in &libs {
        let current = libs_downloaded.load(Ordering::Relaxed);
        emit_progress(
            app,
            "libraries",
            current,
            total_libs,
            &format!("Библиотеки: {}/{}", current, total_libs),
        );

        if let Some(lib_downloads) = &lib.downloads {
            // Download main artifact
            if let Some(artifact) = &lib_downloads.artifact {
                let dest = libs_dir.join(&artifact.path);
                download_file_to(
                    client,
                    &artifact.url,
                    &dest,
                    &artifact.sha1,
                    libs_downloaded.clone(),
                )
                .await?;
            }

            // Download and extract native classifiers
            let native_key = "natives-windows";
            if let Some(natives_map) = &lib.natives {
                if let Some(classifier_key) = natives_map.get("windows") {
                    if let Some(classifiers) = &lib_downloads.classifiers {
                        if let Some(native_artifact) = classifiers.get(classifier_key) {
                            let native_jar_dest = libs_dir.join(&native_artifact.path);
                            download_file_to(
                                client,
                                &native_artifact.url,
                                &native_jar_dest,
                                &native_artifact.sha1,
                                libs_downloaded.clone(),
                            )
                            .await?;

                            // Extract .dll files
                            extract_natives(&native_jar_dest, &natives_dir, &lib.extract)?;
                        }
                    }
                }
            } else {
                // Check if artifact itself is a native (newer versions)
                if let Some(artifact) = &lib_downloads.artifact {
                    if artifact.path.contains(native_key) {
                        let dest = libs_dir.join(&artifact.path);
                        extract_natives(&dest, &natives_dir, &lib.extract)?;
                    }
                }
            }
        }

        libs_downloaded.fetch_add(1, Ordering::Relaxed);
    }

    emit_progress(
        app,
        "libraries",
        total_libs,
        total_libs,
        "Библиотеки установлены",
    );

    // ── 5. Download asset index ────────────────────────────────────────────────
    if let Some(asset_index_info) = &version_json.asset_index {
        emit_progress(app, "assets", 0, 1, "Загрузка индекса ассетов...");
        let asset_indexes_dir = game_dir.join("assets").join("indexes");
        fs::create_dir_all(&asset_indexes_dir).map_err(|e| e.to_string())?;

        let asset_index_path = asset_indexes_dir.join(format!("{}.json", asset_index_info.id));
        let asset_index_text = client
            .get(&asset_index_info.url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        fs::write(&asset_index_path, &asset_index_text).map_err(|e| e.to_string())?;

        let asset_index: AssetIndex =
            serde_json::from_str(&asset_index_text).map_err(|e| e.to_string())?;

        // ── 6. Download assets ─────────────────────────────────────────────────────
        let assets_objects_dir = game_dir.join("assets").join("objects");
        let total_assets = asset_index.objects.len() as u64;
        let assets_downloaded = Arc::new(AtomicU64::new(0));

        let mut tasks = Vec::new();
        for (_, obj) in &asset_index.objects {
            let prefix = &obj.hash[..2];
            let dest = assets_objects_dir.join(prefix).join(&obj.hash);
            let url = format!(
                "https://resources.download.minecraft.net/{}/{}",
                prefix, obj.hash
            );

            let client_c = client.clone();
            let hash_c = obj.hash.clone();
            let dl_c = assets_downloaded.clone();

            tasks
                .push(async move { download_file_to(&client_c, &url, &dest, &hash_c, dl_c).await });
        }

        use futures_util::StreamExt;
        let mut stream = futures_util::stream::iter(tasks).buffer_unordered(50); // 50 concurrent downloads

        while let Some(res) = stream.next().await {
            res?;
            let current = assets_downloaded.load(Ordering::Relaxed);
            if current % 50 == 0 || current == total_assets {
                emit_progress(
                    app,
                    "assets",
                    current,
                    total_assets,
                    &format!("Ассеты: {}/{}", current, total_assets),
                );
            }
        }

        emit_progress(
            app,
            "assets",
            total_assets,
            total_assets,
            "Ассеты установлены",
        );
    }

    Ok(())
}

fn extract_natives(
    jar_path: &PathBuf,
    natives_dir: &PathBuf,
    extract_info: &Option<ExtractInfo>,
) -> Result<(), String> {
    if !jar_path.exists() {
        return Ok(());
    }

    let file = fs::File::open(jar_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let excludes: Vec<String> = extract_info
        .as_ref()
        .and_then(|e| e.exclude.clone())
        .unwrap_or_default();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();

        // Skip excluded paths and directories
        if name.ends_with('/') {
            continue;
        }
        if excludes
            .iter()
            .any(|ex| name.starts_with(ex.trim_end_matches('/')))
        {
            continue;
        }
        // Only extract .dll, .so, .dylib
        if !name.ends_with(".dll") && !name.ends_with(".so") && !name.ends_with(".dylib") {
            continue;
        }

        let filename = std::path::Path::new(&name).file_name().unwrap_or_default();
        let dest = natives_dir.join(filename);

        let mut out = fs::File::create(&dest).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn launch_minecraft(
    app: AppHandle,
    version_id: String,
    account: Account,
    ram_mb: Option<u64>,
    java_preference: Option<String>,
    game_dir_override: Option<String>,
) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let forge_parsed = parse_forge_display(&version_id);
    let fabric_parsed = parse_fabric_display(&version_id);
    let neoforge_parsed = parse_neoforge_display(&version_id);
    let (real_id, is_optifine) = if let Some((ref forge_dir_id, _, _, optifine)) = forge_parsed {
        (forge_dir_id.clone(), optifine)
    } else if let Some((ref fabric_dir_id, _, _)) = fabric_parsed {
        (fabric_dir_id.clone(), false)
    } else if let Some((ref neoforge_dir_id, _, _)) = neoforge_parsed {
        (neoforge_dir_id.clone(), false)
    } else {
        (version_id.clone(), false)
    };
    let real_id = real_id.as_str();

    let install_dir = get_game_dir();
    // If launching a modpack (game_dir_override points to a modpack folder), don't launch directly from the modpack folder
    // because Minecraft will create saves, screenshots and other folders there. Instead create a temporary instance
    // directory under %APPDATA%/.femboy/instances/<modpack>-<timestamp> and copy the modpack contents into it, then
    // launch the game from that temp directory and clean it up after the game exits.
    let mut temp_instance_dir: Option<PathBuf> = None;
    let run_dir = if let Some(ref override_path) = game_dir_override {
        let pack_dir = PathBuf::from(override_path);
        // Create instances root
        let instances_root = get_femboy_dir().join("instances");
        if let Err(_) = fs::create_dir_all(&instances_root) {}
        let ts = chrono::Utc::now().timestamp();
        let pack_name = pack_dir
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "modpack".to_string());
        let temp_dir = instances_root.join(format!("{}-{}", pack_name, ts));
        fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

        // Copy pack contents to temp_dir, but never copy large/managed folders
        for entry in fs::read_dir(&pack_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if matches!(name.as_str(), "versions" | "assets" | "libraries" | "natives" | "modpacks") {
                continue;
            }
            let src = entry.path();
            let dst = temp_dir.join(entry.file_name());
            if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                copy_dir_recursive(&src, &dst).map_err(|e| e.to_string())?;
            } else {
                fs::copy(&src, &dst).map_err(|e| e.to_string())?;
            }
        }

        temp_instance_dir = Some(temp_dir.clone());
        temp_dir
    } else {
        get_game_dir()
    };
    let version_dir = install_dir.join("versions").join(real_id);
    let version_json_path = version_dir.join(format!("{}.json", real_id));

    if !version_json_path.exists() {
        return Err(format!("Версия {} не установлена", version_id));
    }

    let version_json_text = fs::read_to_string(&version_json_path).map_err(|e| e.to_string())?;
    let version_json: VersionJson =
        serde_json::from_str(&version_json_text).map_err(|e| e.to_string())?;

    // ── Resolve inheritsFrom (Forge 1.13+ pattern) ─────────────────────────────
    // Forge 1.13+ version JSONs only list Forge-specific libs and inherit everything
    // else from the vanilla version JSON. We must merge both to build a complete classpath.
    let (merged_libraries, merged_arguments, merged_minecraft_arguments, resolved_asset_index) =
        if let Some(ref parent_id) = version_json.inherits_from {
            let parent_json_path = install_dir
                .join("versions")
                .join(parent_id)
                .join(format!("{}.json", parent_id));
            if parent_json_path.exists() {
                let parent_text =
                    fs::read_to_string(&parent_json_path).map_err(|e| e.to_string())?;
                let parent: VersionJson =
                    serde_json::from_str(&parent_text).map_err(|e| e.to_string())?;

                // Parent libs first, then Forge-specific libs on top
                let mut libs = parent.libraries;
                libs.extend(version_json.libraries);

                // Merge game arguments: parent args + forge args
                let merged_args = match (parent.arguments, version_json.arguments) {
                    (Some(mut p), Some(c)) => {
                        if let Some(cg) = c.game {
                            p.game.get_or_insert_with(Vec::new).extend(cg);
                        }
                        if let Some(cj) = c.jvm {
                            p.jvm.get_or_insert_with(Vec::new).extend(cj);
                        }
                        Some(p)
                    }
                    (Some(p), None) => Some(p),
                    (None, Some(c)) => Some(c),
                    _ => None,
                };

                let mc_args = version_json
                    .minecraft_arguments
                    .or(parent.minecraft_arguments);
                let asset_idx = version_json.asset_index.or(parent.asset_index);

                (libs, merged_args, mc_args, asset_idx)
            } else {
                return Err(format!(
                    "Родительская версия '{}' не найдена. Сначала установите vanilla {}.",
                    parent_id, parent_id
                ));
            }
        } else {
            (
                version_json.libraries,
                version_json.arguments,
                version_json.minecraft_arguments,
                version_json.asset_index,
            )
        };

    let libs_dir = install_dir.join("libraries");
    let mut natives_dir = install_dir.join("natives").join(real_id);
    let assets_dir = install_dir.join("assets");

    // If natives_dir doesn't exist or is empty, fallback to parent's natives_dir
    let parent_fallback = version_json
        .inherits_from
        .clone()
        .or_else(|| {
            forge_parsed
                .as_ref()
                .map(|(_, mc_ver, _, _)| mc_ver.clone())
        })
        .or_else(|| fabric_parsed.as_ref().map(|(_, mc_ver, _)| mc_ver.clone()))
        .or_else(|| {
            neoforge_parsed
                .as_ref()
                .map(|(_, mc_ver, _)| mc_ver.clone())
        });

    if let Some(parent_id) = parent_fallback {
        if !natives_dir.exists()
            || fs::read_dir(&natives_dir)
                .map(|mut d| d.next().is_none())
                .unwrap_or(true)
        {
            natives_dir = install_dir.join("natives").join(&parent_id);
        }
    }

    // jar field or inheritsFrom points to the vanilla client jar
    let jar_name = version_json
        .jar
        .or_else(|| version_json.inherits_from.clone())
        .unwrap_or_else(|| real_id.to_string());
    let mc_version = version_json.inherits_from.clone().unwrap_or_else(|| real_id.to_string());
    let client_jar = install_dir
        .join("versions")
        .join(&mc_version)
        .join(format!("{}.jar", mc_version));

    // Build classpath from merged libraries
    let mut classpath_parts: Vec<String> = Vec::new();
    let mut seen_paths: std::collections::HashSet<String> = std::collections::HashSet::new();

    for lib in &merged_libraries {
        if !should_use_library(lib) {
            continue;
        }
        if let Some(path) = get_library_path(lib) {
            let full = libs_dir.join(&path);
            let key = full.to_string_lossy().into_owned();
            if full.exists() && seen_paths.insert(key.clone()) {
                classpath_parts.push(key);
            }
        }
    }

    let uses_forge_module_launcher = forge_parsed.is_some()
        && version_json.inherits_from.is_some()
        && version_json
            .main_class
            .contains("cpw.mods.bootstraplauncher.BootstrapLauncher");

    if neoforge_parsed.is_none() && !uses_forge_module_launcher {
        classpath_parts.push(client_jar.to_string_lossy().into_owned());
    }
    #[cfg(target_os = "windows")]
    let sep = ";";
    #[cfg(not(target_os = "windows"))]
    let sep = ":";

    let classpath = classpath_parts.join(sep);

    let (launch_username, player_uuid, access_token, user_type) = match &account {
        Account::Offline { username } => {
            let uuid = generate_offline_uuid(&username);
            (
                username.clone(),
                uuid,
                "0".to_string(),
                "legacy".to_string(),
            )
        }
        Account::Ely {
            username,
            uuid,
            access_token,
        } => {
            let formatted_uuid = uuid.replace("-", "");
            (
                username.clone(),
                formatted_uuid,
                access_token.clone(),
                "mojang".to_string(),
            )
        }
        Account::Microsoft {
            username,
            uuid,
            access_token,
        } => {
            let formatted_uuid = uuid.replace("-", "");
            (
                username.clone(),
                formatted_uuid,
                access_token.clone(),
                "msa".to_string(),
            )
        }
    };
    let assets_index_id = resolved_asset_index
        .as_ref()
        .map(|a| a.id.clone())
        .unwrap_or_else(|| jar_name.clone());
    let mc_version_for_java = fabric_parsed
        .as_ref()
        .map(|(_, mc_ver, _)| mc_ver.clone())
        .or_else(|| {
            forge_parsed
                .as_ref()
                .map(|(_, mc_ver, _, _)| mc_ver.clone())
        })
        .or_else(|| {
            neoforge_parsed
                .as_ref()
                .map(|(_, mc_ver, _)| mc_ver.clone())
        })
        .or_else(|| version_json.inherits_from.clone())
        .unwrap_or_else(|| real_id.to_string());
    let java_command = select_java_command(
        &app,
        &client,
        &mc_version_for_java,
        java_preference.as_deref(),
    )
    .await?;
    let game_dir_str = run_dir.to_string_lossy().into_owned();
    let assets_dir_str = assets_dir.to_string_lossy().into_owned();
    let natives_dir_str = natives_dir.to_string_lossy().into_owned();
    let libs_dir_str = libs_dir.to_string_lossy().into_owned();

    // Build JVM + Game arguments
    let mut args: Vec<String> = Vec::new();

    // If it's an Ely account, we need authlib-injector for skins and auth to work correctly in-game
    if let Account::Ely { .. } = account {
        let injector_path = install_dir.join("authlib-injector.jar");
        if !injector_path.exists() {
            emit_progress(
                &app,
                "authlib-injector",
                0,
                1,
                "Загрузка authlib-injector для скинов...",
            );
            let client = reqwest::Client::new();
            let resp = client.get("https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.7/authlib-injector-1.2.7.jar")
                .send()
                .await
                .map_err(|e| format!("Ошибка загрузки authlib-injector: {}", e))?;

            if !resp.status().is_success() {
                return Err(format!(
                    "Ошибка загрузки authlib-injector: статус {}",
                    resp.status()
                ));
            }

            let bytes = resp
                .bytes()
                .await
                .map_err(|e| format!("Ошибка чтения authlib-injector: {}", e))?;
            fs::write(&injector_path, bytes)
                .map_err(|e| format!("Ошибка сохранения authlib-injector: {}", e))?;
            emit_progress(&app, "authlib-injector", 1, 1, "authlib-injector загружен");
        }
        let injector_arg = format!(
            "-javaagent:{}={}",
            injector_path.to_string_lossy(),
            "https://authserver.ely.by"
        );
        args.push(injector_arg);
    }

    // Base JVM args
    args.push("-Dminecraft.launcher.brand=femboy".into());
    args.push("-Dminecraft.launcher.version=1.0".into());
    args.push(format!("-Djava.library.path={}", natives_dir_str));
    let ram = ram_mb.unwrap_or(2048).clamp(1024, 16384);
    args.push(format!("-Xmx{}M", ram));
    args.push("-Xms512M".into());
    args.push("-XX:+UnlockExperimentalVMOptions".into());
    args.push("-XX:+UseG1GC".into());

    let has_custom_jvm_args = merged_arguments.as_ref().map_or(false, |a| a.jvm.is_some());

    if has_custom_jvm_args {
        if let Some(arguments) = &merged_arguments {
            if let Some(jvm_args) = &arguments.jvm {
                for arg in jvm_args {
                    if let Some(s) = arg.as_str() {
                        let resolved = resolve_arg(
                            s,
                            &launch_username,
                            &player_uuid,
                            &version_id,
                            &game_dir_str,
                            &assets_dir_str,
                            &assets_index_id,
                            &libs_dir_str,
                            &classpath,
                            &natives_dir_str,
                            &access_token,
                            &user_type,
                        );
                        args.push(resolved);
                    }
                }
            }
        }
    } else {
        args.push("-cp".into());
        args.push(classpath.clone());
    }

    args.push(version_json.main_class.clone());

    // Game args — modern (1.13+) or legacy (1.6–1.12)
    if let Some(arguments) = &merged_arguments {
        if let Some(game_args) = &arguments.game {
            for arg in game_args {
                if let Some(s) = arg.as_str() {
                    let resolved = resolve_arg(
                        s,
                        &launch_username,
                        &player_uuid,
                        &version_id,
                        &game_dir_str,
                        &assets_dir_str,
                        &assets_index_id,
                        &libs_dir_str,
                        &classpath,
                        &natives_dir_str,
                        &access_token,
                        &user_type,
                    );
                    args.push(resolved);
                }
            }
        }
    } else if let Some(legacy_args) = &merged_minecraft_arguments {
        for token in legacy_args.split_whitespace() {
            let resolved = resolve_arg(
                token,
                &launch_username,
                &player_uuid,
                &version_id,
                &game_dir_str,
                &assets_dir_str,
                &assets_index_id,
                &libs_dir_str,
                &classpath,
                &natives_dir_str,
                &access_token,
                &user_type,
            );
            args.push(resolved);
        }
    }

    let debug_log_path = run_dir.join("launch_debug.log");
    let mut debug_log = fs::File::create(&debug_log_path).map_err(|e| e.to_string())?;

    let mods_dir = run_dir.join("mods");
    if let Ok(entries) = fs::read_dir(&mods_dir) {
        let expected_prefix = if is_optifine {
            if let Some((_, ref mc_ver, _, _)) = forge_parsed {
                Some(format!("OptiFine_{}_", mc_ver))
            } else {
                None
            }
        } else {
            None
        };

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("OptiFine_") && name.ends_with(".jar") {
                let should_keep = match &expected_prefix {
                    Some(prefix) => name.starts_with(prefix),
                    None => false,
                };
                if !should_keep {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }

    if is_optifine {
        if let Some((_, ref mc_ver, _, _)) = forge_parsed {
            emit_progress(&app, "optifine", 0, 1, "Проверка Optifine...");
            if let Err(e) = ensure_optifine(&run_dir, mc_ver).await {
                let _ = writeln!(debug_log, "[WARNING] Не удалось скачать Optifine: {}", e);
            } else {
                emit_progress(&app, "optifine", 1, 1, "Optifine установлен!");
            }
        }
    }

    let args_str = args.join("\n");
    let _ = writeln!(
        debug_log,
        "Java command:\n{}\n\nLaunch arguments:\n{}\n\n--- Output ---",
        java_command, args_str
    );

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    // If a temporary instance dir was created for a modpack launch, remember it so we can clean it up after the game exits
    let cleanup_path = temp_instance_dir.clone();

    let (mut rx, _child) = app
        .shell()
        .command(&java_command) // use java instead of javaw to capture output properly in some cases
        .args(&args_refs)
        .current_dir(&run_dir)
        .spawn()
        .map_err(|e| e.to_string())?;

    let app_clone = app.clone();
    if let Some(window) = app_clone.get_webview_window("main") {
        let _ = window.hide();
    }

    let launch_start_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                    let _ = writeln!(debug_log, "[STDOUT] {}", String::from_utf8_lossy(&line));
                }
                tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                    let _ = writeln!(debug_log, "[STDERR] {}", String::from_utf8_lossy(&line));
                }
                tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                    let _ = writeln!(debug_log, "[TERMINATED] code: {:?}", payload.code);
                    
                    let launch_end_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                        
                    let duration = launch_end_time.saturating_sub(launch_start_time);
                    if duration > 0 {
                        append_playtime_session(PlaySession {
                            start: launch_start_time,
                            duration,
                        });
                    }

                    // Clean up temporary instance directory created for modpack launches
                    if let Some(p) = cleanup_path.clone() {
                        let _ = fs::remove_dir_all(p);
                    }
                    if let Some(rpc) = app_clone.try_state::<Arc<rpc::Rpc>>() {
                        rpc.set_activity("В лаунчере", "Просматривает главную страницу");
                    }
                    app_clone.emit("game_closed", ()).ok();
                    break;
                }
                tauri_plugin_shell::process::CommandEvent::Error(err) => {
                    let _ = writeln!(debug_log, "[ERROR] {}", err);
                    break;
                }
                _ => {}
            }
        }
        if let Some(window) = app_clone.get_webview_window("main") {
            let _ = window.show();
        }
    });

    Ok(())
}

fn resolve_arg(
    template: &str,
    username: &str,
    uuid: &str,
    version: &str,
    game_dir: &str,
    assets_dir: &str,
    asset_index: &str,
    libs_dir: &str,
    classpath: &str,
    natives_dir: &str,
    access_token: &str,
    user_type: &str,
) -> String {
    let mut resolved = template
        .replace("${auth_player_name}", username)
        .replace("${auth_uuid}", uuid)
        .replace("${auth_access_token}", access_token)
        .replace(
            "${auth_session}",
            &format!("token:{}:{}", access_token, uuid),
        )
        .replace("${user_type}", user_type)
        .replace("${user_properties}", "{}")
        .replace("${version_name}", version)
        .replace("${game_directory}", game_dir)
        .replace("${assets_root}", assets_dir)
        .replace("${assets_index_name}", asset_index)
        .replace("${game_assets}", assets_dir)
        .replace("${version_type}", "release")
        .replace("${launcher_name}", "femboy")
        .replace("${launcher_version}", "1.0")
        .replace("${natives_directory}", natives_dir)
        .replace("${library_directory}", libs_dir)
        .replace("${classpath}", classpath)
        .replace("${classpath_resolv}", classpath);

    #[cfg(target_os = "windows")]
    {
        resolved = resolved.replace("${classpath_separator}", ";");
    }
    #[cfg(not(target_os = "windows"))]
    {
        resolved = resolved.replace("${classpath_separator}", ":");
    }

    resolved
}

fn emit_progress(app: &AppHandle, stage: &str, current: u64, total: u64, message: &str) {
    let _ = app.emit(
        "download_progress",
        ProgressEvent {
            stage: stage.to_string(),
            current,
            total,
            message: message.to_string(),
        },
    );
}

#[derive(serde::Deserialize, Debug)]
struct OptifineVersion {
    mcversion: String,
    patch: String,
    #[serde(rename = "type")]
    type_: String,
    filename: String,
}

async fn ensure_optifine(game_dir: &PathBuf, mc_ver: &str) -> Result<(), String> {
    let mods_dir = game_dir.join("mods");
    fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;

    // Check if optifine for this mc_ver already exists
    let mut exists = false;
    if let Ok(entries) = fs::read_dir(&mods_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&format!("OptiFine_{}_", mc_ver)) && name.ends_with(".jar") {
                exists = true;
                break;
            }
        }
    }

    if exists {
        return Ok(());
    }

    // Need to download
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let list: Vec<OptifineVersion> = client
        .get("https://bmclapi2.bangbang93.com/optifine/versionList")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    // Find first matching version
    let optifine_info = list
        .into_iter()
        .find(|v| v.mcversion == mc_ver)
        .ok_or_else(|| format!("OptiFine для версии {} не найден", mc_ver))?;

    let url = format!(
        "https://bmclapi2.bangbang93.com/optifine/{}/{}/{}",
        optifine_info.mcversion, optifine_info.type_, optifine_info.patch
    );

    let dest = mods_dir.join(&optifine_info.filename);

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    fs::write(&dest, &bytes).map_err(|e| e.to_string())?;

    Ok(())
}

// ─── Modpack Commands ────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModpackMeta {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub version: String,
    pub avatar: Option<String>,
    pub created_at: String,
}

fn get_modpacks_dir() -> PathBuf {
    get_game_dir().join("modpacks")
}

#[tauri::command]
fn list_modpacks() -> Result<Vec<ModpackMeta>, String> {
    let dir = get_modpacks_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut packs: Vec<ModpackMeta> = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let meta_path = path.join("modpack.json");
            if meta_path.exists() {
                if let Ok(content) = fs::read_to_string(&meta_path) {
                    if let Ok(meta) = serde_json::from_str::<ModpackMeta>(&content) {
                        packs.push(meta);
                    }
                }
            }
        }
    }
    packs.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(packs)
}

#[tauri::command]
async fn create_modpack(
    id: String,
    name: String,
    description: Option<String>,
    author: Option<String>,
    version: String,
    mode: String,
    snapshot_folders: Vec<String>,
) -> Result<(), String> {
    let modpacks_dir = get_modpacks_dir();
    let pack_dir = modpacks_dir.join(&id);
    fs::create_dir_all(&pack_dir).map_err(|e| e.to_string())?;

    let now = chrono::Utc::now().to_rfc3339();
    let meta = ModpackMeta {
        id: id.clone(),
        name,
        description,
        author,
        version,
        avatar: None,
        created_at: now,
    };
    let content = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(pack_dir.join("modpack.json"), content).map_err(|e| e.to_string())?;

    if mode == "snapshot" {
        let game_dir = get_game_dir();
        for folder in &snapshot_folders {
            // Whitelist: never copy versions, assets, libraries, natives
            if matches!(folder.as_str(), "versions" | "assets" | "libraries" | "natives" | "modpacks") {
                continue;
            }
            let src = game_dir.join(folder);
            let dst = pack_dir.join(folder);
            if src.exists() {
                copy_dir_recursive(&src, &dst).map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[tauri::command]
fn delete_modpack(id: String) -> Result<(), String> {
    let pack_dir = get_modpacks_dir().join(&id);
    if pack_dir.exists() {
        fs::remove_dir_all(&pack_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn update_modpack(
    id: String,
    name: String,
    description: Option<String>,
    author: Option<String>,
    version: String,
) -> Result<(), String> {
    let pack_dir = get_modpacks_dir().join(&id);
    let meta_path = pack_dir.join("modpack.json");
    if !meta_path.exists() {
        return Err(format!("Modpack '{}' not found", id));
    }
    let content = fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
    let mut meta: ModpackMeta = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    meta.name = name;
    meta.description = description;
    meta.author = author;
    meta.version = version;
    // Keep avatar and created_at unchanged
    let updated = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(&meta_path, updated).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn save_modpack_avatar(id: String, image_data: String) -> Result<(), String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let pack_dir = get_modpacks_dir().join(&id);
    let meta_path = pack_dir.join("modpack.json");
    // Strip data URL prefix if present
    let b64 = if let Some(pos) = image_data.find(',') {
        &image_data[pos + 1..]
    } else {
        &image_data
    };
    let bytes = STANDARD.decode(b64).map_err(|e| e.to_string())?;
    fs::write(pack_dir.join("avatar.png"), &bytes).map_err(|e| e.to_string())?;
    // Update meta to mark avatar present
    if meta_path.exists() {
        if let Ok(content) = fs::read_to_string(&meta_path) {
            if let Ok(mut meta) = serde_json::from_str::<ModpackMeta>(&content) {
                meta.avatar = Some("avatar.png".to_string());
                if let Ok(updated) = serde_json::to_string_pretty(&meta) {
                    let _ = fs::write(&meta_path, updated);
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn get_modpack_avatar(id: String) -> Result<Option<String>, String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let pack_dir = get_modpacks_dir().join(&id);
    let avatar_path = pack_dir.join("avatar.png");
    if avatar_path.exists() {
        let bytes = fs::read(&avatar_path).map_err(|e| e.to_string())?;
        let b64 = STANDARD.encode(&bytes);
        Ok(Some(format!("data:image/png;base64,{}", b64)))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn get_modpack_path(id: String) -> String {
    get_modpacks_dir().join(&id).to_string_lossy().into_owned()
}

#[tauri::command]
fn open_modpack_folder(id: String) -> Result<(), String> {
    let pack_dir = get_modpacks_dir().join(&id);
    if !pack_dir.exists() {
        return Err(format!("Modpack folder '{}' does not exist", id));
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(pack_dir.to_string_lossy().as_ref())
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(pack_dir.to_string_lossy().as_ref())
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(pack_dir.to_string_lossy().as_ref())
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn export_modpack(id: String, dest_path: String) -> Result<(), String> {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    let pack_dir = get_modpacks_dir().join(&id);
    if !pack_dir.exists() {
        return Err(format!("Modpack '{}' not found", id));
    }
    
    let tar_gz = fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);
    
    tar.append_dir_all("", &pack_dir).map_err(|e| e.to_string())?;
    tar.finish().map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
fn import_modpack(archive_path: String) -> Result<(), String> {
    use flate2::read::GzDecoder;

    let file = fs::File::open(&archive_path).map_err(|e| e.to_string())?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    
    let mut meta: Option<ModpackMeta> = None;
    if let Ok(mut entries) = archive.entries() {
        while let Some(Ok(mut entry)) = entries.next() {
            if let Ok(path) = entry.path() {
                let path_str = path.to_string_lossy().to_string();
                if path_str == "modpack.json" || path_str.ends_with("/modpack.json") || path_str.ends_with("\\modpack.json") {
                    let mut content = String::new();
                    use std::io::Read;
                    if entry.read_to_string(&mut content).is_ok() {
                        if let Ok(parsed) = serde_json::from_str::<ModpackMeta>(&content) {
                            meta = Some(parsed);
                        }
                    }
                    break;
                }
            }
        }
    }
    
    let meta = meta.ok_or_else(|| "модпак не поддерживается".to_string())?;
    
    let dest_dir = get_modpacks_dir().join(&meta.id);
    
    let final_id = if dest_dir.exists() {
        format!("{}-{}", meta.id, chrono::Utc::now().timestamp())
    } else {
        meta.id.clone()
    };
    
    let final_dir = get_modpacks_dir().join(&final_id);
    fs::create_dir_all(&final_dir).map_err(|e| e.to_string())?;
    
    let file = fs::File::open(&archive_path).map_err(|e| e.to_string())?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    archive.unpack(&final_dir).map_err(|e| e.to_string())?;
    
    // Update the ID in modpack.json in the extracted folder
    if final_id != meta.id {
        let mut new_meta = meta.clone();
        new_meta.id = final_id;
        let content = serde_json::to_string_pretty(&new_meta).map_err(|e| e.to_string())?;
        fs::write(final_dir.join("modpack.json"), content).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

// ─── Modpack Mods Data ─────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct ModMeta {
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub icon: Option<String>,
}

fn extract_mod_meta(jar_path: &PathBuf) -> Option<ModMeta> {
    use std::io::Read;
    use base64::Engine;
    let file = fs::File::open(jar_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    // Try Fabric
    let fabric_content = archive.by_name("fabric.mod.json").ok().and_then(|mut f| {
        let mut s = String::new();
        f.read_to_string(&mut s).ok()?;
        Some(s)
    });
    if let Some(contents) = fabric_content {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&contents) {
            let id = val.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            let name = val.get("name").and_then(|v| v.as_str()).unwrap_or(&id).to_string();
            let author = val.get("authors").and_then(|v| v.as_array()).and_then(|a| a.first()).and_then(|v| v.as_str()).map(|s| s.to_string());
            let icon_path = val.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());
            
            let icon = icon_path.and_then(|p| {
                if let Ok(mut icon_file) = archive.by_name(p.trim_start_matches('/')) {
                    let mut buf = Vec::new();
                    if icon_file.read_to_end(&mut buf).is_ok() {
                        return Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&buf)));
                    }
                }
                None
            });
            return Some(ModMeta { id, name, author, icon });
        }
    }

    // Try Quilt
    let quilt_content = archive.by_name("quilt.mod.json").ok().and_then(|mut f| {
        let mut s = String::new();
        f.read_to_string(&mut s).ok()?;
        Some(s)
    });
    if let Some(contents) = quilt_content {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&contents) {
            if let Some(metadata) = val.get("quilt_loader").and_then(|q| q.get("metadata")) {
                let id = metadata.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                let name = metadata.get("name").and_then(|v| v.as_str()).unwrap_or(&id).to_string();
                let author = metadata.get("contributors").and_then(|v| v.as_object()).and_then(|m| m.keys().next()).map(|s| s.to_string());
                let icon_path = metadata.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());
                
                let icon = icon_path.and_then(|p| {
                    if let Ok(mut icon_file) = archive.by_name(p.trim_start_matches('/')) {
                        let mut buf = Vec::new();
                        if icon_file.read_to_end(&mut buf).is_ok() {
                            return Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&buf)));
                        }
                    }
                    None
                });
                return Some(ModMeta { id, name, author, icon });
            }
        }
    }

    // Try Forge / NeoForge
    for toml_path in &["META-INF/mods.toml", "META-INF/neoforge.mods.toml"] {
        let toml_content = archive.by_name(toml_path).ok().and_then(|mut f| {
            let mut s = String::new();
            f.read_to_string(&mut s).ok()?;
            Some(s)
        });
        if let Some(contents) = toml_content {
            if let Ok(val) = contents.parse::<toml::Value>() {
                if let Some(mods) = val.get("mods").and_then(|v| v.as_array()).and_then(|a| a.first()) {
                    let id = mods.get("modId").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                    let name = mods.get("displayName").and_then(|v| v.as_str()).unwrap_or(&id).to_string();
                    let author = mods.get("authors").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let icon_path = mods.get("logoFile").and_then(|v| v.as_str()).map(|s| s.to_string());
                    
                    let icon = icon_path.and_then(|p| {
                        if let Ok(mut icon_file) = archive.by_name(p.trim_start_matches('/')) {
                            let mut buf = Vec::new();
                            if icon_file.read_to_end(&mut buf).is_ok() {
                                return Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&buf)));
                            }
                        }
                        None
                    });
                    return Some(ModMeta { id, name, author, icon });
                }
            }
        }
    }

    // Try Legacy Forge
    let mcmod_content = archive.by_name("mcmod.info").ok().and_then(|mut f| {
        let mut s = String::new();
        f.read_to_string(&mut s).ok()?;
        Some(s)
    });
    if let Some(contents) = mcmod_content {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&contents) {
            let mods_array = if val.is_array() {
                val.as_array()
            } else if let Some(ml) = val.get("modList").and_then(|v| v.as_array()) {
                Some(ml)
            } else {
                None
            };
            
            if let Some(mods) = mods_array.and_then(|a| a.first()) {
                let id = mods.get("modid").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                let name = mods.get("name").and_then(|v| v.as_str()).unwrap_or(&id).to_string();
                let author = mods.get("authorList").and_then(|v| v.as_array()).and_then(|a| a.first()).and_then(|v| v.as_str()).map(|s| s.to_string());
                let icon_path = mods.get("logoFile").and_then(|v| v.as_str()).map(|s| s.to_string());
                
                let icon = icon_path.and_then(|p| {
                    if !p.is_empty() {
                        if let Ok(mut icon_file) = archive.by_name(p.trim_start_matches('/')) {
                            let mut buf = Vec::new();
                            if icon_file.read_to_end(&mut buf).is_ok() {
                                return Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&buf)));
                            }
                        }
                    }
                    None
                });
                return Some(ModMeta { id, name, author, icon });
            }
        }
    }

    None
}

#[tauri::command]
fn get_modpack_mods(id: String) -> Result<Vec<ModMeta>, String> {
    let mods_dir = get_modpacks_dir().join(&id).join("mods");
    let mut mods = Vec::new();
    
    if mods_dir.exists() {
        if let Ok(entries) = fs::read_dir(mods_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().unwrap_or_default() == "jar" {
                    if let Some(meta) = extract_mod_meta(&path) {
                        mods.push(meta);
                    } else {
                        // Fallback if no meta found
                        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        mods.push(ModMeta {
                            id: file_name.clone(),
                            name: file_name,
                            author: None,
                            icon: None,
                        });
                    }
                }
            }
        }
    }
    
    mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(mods)
}

// ─── Mod File List & Toggle ────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct ModFile {
    pub filename: String,
    pub enabled: bool,
    pub meta: Option<ModMeta>,
}

#[tauri::command]
fn list_mods(modpack_id: String) -> Result<Vec<ModFile>, String> {
    let mods_dir = if modpack_id == "global" {
        get_game_dir().join("mods")
    } else {
        get_modpacks_dir().join(&modpack_id).join("mods")
    };
    let mut result = Vec::new();

    if !mods_dir.exists() {
        return Ok(result);
    }

    if let Ok(entries) = fs::read_dir(&mods_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() { continue; }
            let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let enabled;
            let jar_path;
            if fname.ends_with(".jar") {
                enabled = true;
                jar_path = path.clone();
            } else if fname.ends_with(".jar.disabled") {
                enabled = false;
                jar_path = path.clone();
            } else {
                continue;
            }
            let meta = extract_mod_meta(&jar_path);
            result.push(ModFile { filename: fname, enabled, meta });
        }
    }

    result.sort_by(|a, b| {
        let name_a = a.meta.as_ref().map(|m| m.name.to_lowercase()).unwrap_or_else(|| a.filename.to_lowercase());
        let name_b = b.meta.as_ref().map(|m| m.name.to_lowercase()).unwrap_or_else(|| b.filename.to_lowercase());
        name_a.cmp(&name_b)
    });
    Ok(result)
}

#[tauri::command]
fn toggle_mod(modpack_id: String, filename: String) -> Result<String, String> {
    let mods_dir = if modpack_id == "global" {
        get_game_dir().join("mods")
    } else {
        get_modpacks_dir().join(&modpack_id).join("mods")
    };
    let current_path = mods_dir.join(&filename);

    if !current_path.exists() {
        return Err(format!("File not found: {}", filename));
    }

    let new_filename = if filename.ends_with(".jar.disabled") {
        filename[..filename.len() - ".disabled".len()].to_string()
    } else if filename.ends_with(".jar") {
        format!("{}.disabled", filename)
    } else {
        return Err(format!("Unknown file extension: {}", filename));
    };

    let new_path = mods_dir.join(&new_filename);
    fs::rename(&current_path, &new_path).map_err(|e| e.to_string())?;
    Ok(new_filename)
}

// ─── Run ───────────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = std::fs::write("update.log", ""); // Clear update log on startup
    let rpc = Arc::new(rpc::Rpc::new());
    rpc.connect();

    tauri::Builder::default()
        .manage(rpc)
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_versions,
            is_version_installed,
            install_version,
            launch_minecraft,
            get_playtime_stats,
            load_accounts,
            save_accounts,
            login_ely,
            login_microsoft,
            load_settings,
            save_settings,
            get_game_dir_path,
            check_for_updates,
            download_update,
            apply_update,
            set_rpc_activity,
            clear_rpc,
            list_modpacks,
            create_modpack,
            delete_modpack,
            update_modpack,
            save_modpack_avatar,
            get_modpack_avatar,
            get_modpack_path,
            open_modpack_folder,
            export_modpack,
            import_modpack,
            get_modpack_mods,
            list_mods,
            toggle_mod,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
