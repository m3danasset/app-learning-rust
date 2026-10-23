#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

const PORT: u16 = 8787;
// dev panel: while set, the window is allowed to stay out of fullscreen
static WINDOWED: AtomicBool = AtomicBool::new(false);
type Db = Arc<Mutex<Connection>>;
type R<T> = Result<T, String>;

fn err<E: ToString>(x: E) -> String {
    x.to_string()
}

#[derive(Serialize)]
struct User {
    id: i64,
    name: String,
    username: String,
    role: String,
}

#[derive(Serialize)]
struct Course {
    id: i64,
    title: String,
    description: String,
}

fn hash(p: &str) -> R<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default().hash_password(p.as_bytes(), &salt).map(|h| h.to_string()).map_err(err)
}

fn verify(p: &str, h: &str) -> bool {
    PasswordHash::new(h)
        .map(|ph| Argon2::default().verify_password(p.as_bytes(), &ph).is_ok())
        .unwrap_or(false)
}

fn user_count(c: &Connection) -> R<i64> {
    c.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).map_err(err)
}

fn insert_user(c: &Connection, name: &str, username: &str, pw: &str, role: &str) -> R<()> {
    if name.trim().is_empty() || username.trim().is_empty() {
        return Err("Nama dan username wajib diisi".into());
    }
    c.execute(
        "INSERT INTO users(name,username,password_hash,role) VALUES(?1,?2,?3,?4)",
        params![name.trim(), username.trim(), hash(pw)?, role],
    )
    .map(|_| ())
    .map_err(|_| "Username sudah dipakai".to_string())
}

fn fetch_courses(c: &Connection) -> rusqlite::Result<Vec<Course>> {
    let mut s = c.prepare("SELECT id,title,description FROM courses ORDER BY id DESC")?;
    let rows = s.query_map([], |r| {
        Ok(Course { id: r.get(0)?, title: r.get(1)?, description: r.get(2)? })
    })?;
    rows.collect()
}

#[tauri::command]
fn needs_setup(db: State<Db>) -> R<bool> {
    let c = db.lock().map_err(err)?;
    Ok(user_count(&c)? == 0)
}

#[tauri::command]
fn setup_teacher(db: State<Db>, name: String, username: String, password: String) -> R<()> {
    let c = db.lock().map_err(err)?;
    if user_count(&c)? > 0 {
        return Err("Akun guru sudah dibuat".into());
    }
    insert_user(&c, &name, &username, &password, "teacher")
}

#[tauri::command]
fn login(db: State<Db>, username: String) -> R<User> {
    let c = db.lock().map_err(err)?;
    c.query_row(
        "SELECT id,name,username,role FROM users WHERE username=?1 COLLATE NOCASE AND role!='student'",
        [username.trim()],
        |r| Ok(User { id: r.get(0)?, name: r.get(1)?, username: r.get(2)?, role: r.get(3)? }),
    )
    .map_err(|_| "ID not found".to_string())
}

#[tauri::command]
fn list_students(db: State<Db>) -> R<Vec<User>> {
    let c = db.lock().map_err(err)?;
    let mut s = c
        .prepare("SELECT id,name,username,role FROM users WHERE role='student' ORDER BY name")
        .map_err(err)?;
    let rows = s
        .query_map([], |r| {
            Ok(User { id: r.get(0)?, name: r.get(1)?, username: r.get(2)?, role: r.get(3)? })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}

#[tauri::command]
fn list_staff(db: State<Db>) -> R<Vec<User>> {
    let c = db.lock().map_err(err)?;
    let mut s = c
        .prepare("SELECT id,name,username,role FROM users WHERE role!='student' ORDER BY id")
        .map_err(err)?;
    let rows = s
        .query_map([], |r| {
            Ok(User { id: r.get(0)?, name: r.get(1)?, username: r.get(2)?, role: r.get(3)? })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}

#[tauri::command]
fn add_student(db: State<Db>, name: String, username: String, password: Option<String>) -> R<()> {
    let c = db.lock().map_err(err)?;
    insert_user(&c, &name, &username, &password.unwrap_or_default(), "student")
}

#[tauri::command]
fn delete_student(db: State<Db>, id: i64) -> R<()> {
    let c = db.lock().map_err(err)?;
    c.execute("DELETE FROM users WHERE id=?1 AND role='student'", [id]).map(|_| ()).map_err(err)
}

#[tauri::command]
fn list_courses(db: State<Db>) -> R<Vec<Course>> {
    let c = db.lock().map_err(err)?;
    fetch_courses(&c).map_err(err)
}

#[tauri::command]
fn add_course(db: State<Db>, title: String, description: String) -> R<()> {
    if title.trim().is_empty() {
        return Err("Judul kursus wajib diisi".into());
    }
    let c = db.lock().map_err(err)?;
    c.execute(
        "INSERT INTO courses(title,description) VALUES(?1,?2)",
        params![title.trim(), description.trim()],
    )
    .map(|_| ())
    .map_err(err)
}

#[tauri::command]
fn delete_course(db: State<Db>, id: i64) -> R<()> {
    let c = db.lock().map_err(err)?;
    c.execute("DELETE FROM courses WHERE id=?1", [id]).map(|_| ()).map_err(err)
}

#[tauri::command]
fn update_course(db: State<Db>, id: i64, description: String) -> R<()> {
    let c = db.lock().map_err(err)?;
    c.execute("UPDATE courses SET description=?1 WHERE id=?2", params![description.trim(), id])
        .map(|_| ())
        .map_err(err)
}

// videos/ is searched upward from the exe, so it works from target/debug, target/release or next to a copied exe
fn videos_root() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors().skip(1).map(|d| d.join("videos")).find(|d| d.is_dir())
}

#[derive(Serialize)]
struct Lesson {
    title: String,
    video: String,
    sub_id: Option<String>,
    sub_en: Option<String>,
}

#[tauri::command]
fn list_lessons(course: String) -> R<Vec<Lesson>> {
    if course.is_empty() || course.contains(['/', '\\', '.']) {
        return Err("Kursus tidak valid".into());
    }
    let Some(dir) = videos_root().map(|r| r.join(&course)) else { return Ok(vec![]) };
    let Ok(rd) = std::fs::read_dir(&dir) else { return Ok(vec![]) };
    let files: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    let find = |stem: &str, suffix: &str| {
        let want = format!("{}{}.vtt", stem.to_lowercase(), suffix);
        files.iter().find(|p| p.file_name().map_or(false, |n| n.to_string_lossy().to_lowercase() == want))
            .map(|p| p.to_string_lossy().into_owned())
    };
    let mut out: Vec<Lesson> = files.iter()
        .filter(|p| p.extension().map_or(false, |e| e.eq_ignore_ascii_case("mp4")))
        .filter_map(|p| {
            let stem = p.file_stem()?.to_string_lossy().into_owned();
            Some(Lesson { sub_id: find(&stem, "_ind"), sub_en: find(&stem, "_eng"), video: p.to_string_lossy().into_owned(), title: stem })
        })
        .collect();
    // "2. x" before "10. y": order by the leading lesson number, then name
    let num = |t: &str| t.split('.').next().and_then(|n| n.trim().parse::<u32>().ok()).unwrap_or(u32::MAX);
    out.sort_by(|a, b| num(&a.title).cmp(&num(&b.title)).then(a.title.cmp(&b.title)));
    Ok(out)
}

#[tauri::command]
fn read_sub(path: String) -> R<String> {
    let root = videos_root().ok_or("Folder videos tidak ditemukan")?.canonicalize().map_err(err)?;
    let p = std::path::Path::new(&path).canonicalize().map_err(err)?;
    if !p.starts_with(&root) || p.extension().map_or(true, |e| !e.eq_ignore_ascii_case("vtt")) {
        return Err("File tidak diizinkan".into());
    }
    std::fs::read_to_string(p).map_err(err)
}

#[tauri::command]
fn set_view(window: tauri::WebviewWindow, mode: String) -> R<()> {
    let size = match mode.as_str() {
        "fullscreen" => None,
        "768" => Some((1366.0, 768.0)),
        "1080" => Some((1920.0, 1080.0)),
        "1440" => Some((2560.0, 1440.0)),
        _ => return Err("Mode tidak dikenal".into()),
    };
    match size {
        None => {
            WINDOWED.store(false, Ordering::SeqCst);
            window.set_fullscreen(true).map_err(err)
        }
        Some((w, h)) => {
            WINDOWED.store(true, Ordering::SeqCst);
            window.set_fullscreen(false).map_err(err)?;
            window.set_resizable(true).map_err(err)?;
            window.set_size(tauri::LogicalSize::new(w, h)).map_err(err)?;
            window.center().map_err(err)?;
            window.set_resizable(false).map_err(err)
        }
    }
}

#[tauri::command]
fn server_info() -> String {
    let ip = local_ip_address::local_ip().map(|i| i.to_string()).unwrap_or("127.0.0.1".into());
    format!("http://{}:{}", ip, PORT)
}

#[tauri::command]
fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

// Server LAN sederhana untuk aplikasi murid.
fn serve(db: Db) {
    let Ok(srv) = tiny_http::Server::http(("0.0.0.0", PORT)) else { return };
    let header = |k: &str, v: &str| tiny_http::Header::from_bytes(k, v).unwrap();
    for req in srv.incoming_requests() {
        let body = match req.url() {
            "/api/ping" => r#"{"ok":true}"#.to_string(),
            "/api/courses" => db
                .lock()
                .ok()
                .and_then(|c| fetch_courses(&c).ok())
                .and_then(|v| serde_json::to_string(&v).ok())
                .unwrap_or("[]".into()),
            _ => "{}".into(),
        };
        let _ = req.respond(
            tiny_http::Response::from_string(body)
                .with_header(header("Content-Type", "application/json"))
                .with_header(header("Access-Control-Allow-Origin", "*")),
        );
    }
}

fn main() {
    tauri::Builder::default()
        // leaving a video's own fullscreen drops the window out of fullscreen; put it back
        .on_window_event(|w, e| {
            if matches!(e, tauri::WindowEvent::Resized(_)) && !WINDOWED.load(Ordering::SeqCst) && !w.is_fullscreen().unwrap_or(true) {
                let _ = w.set_fullscreen(true);
            }
        })
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let c = Connection::open(dir.join("elearning.db"))?;
            c.execute_batch(
                "CREATE TABLE IF NOT EXISTS users(
                   id INTEGER PRIMARY KEY, name TEXT NOT NULL, username TEXT NOT NULL UNIQUE,
                   password_hash TEXT NOT NULL, role TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS courses(
                   id INTEGER PRIMARY KEY, title TEXT NOT NULL, description TEXT NOT NULL DEFAULT '');
                 INSERT OR IGNORE INTO users(name,username,password_hash,role) VALUES
                   ('Admin','admin001','','admin'),('Guru','guru001','','teacher'),
                   ('PB','pb001','','pb'),('Trial','trial','','trial');
                 DELETE FROM courses WHERE title IN
                   ('Matematika','Bahasa Indonesia','IPA','Bahasa Inggris','IPS')
                   AND description IN ('Bilangan, aljabar, dan geometri dasar.','Membaca, menulis, dan tata bahasa.',
                     'Mengenal alam, makhluk hidup, dan energi.','Kosakata dan percakapan sehari-hari.',
                     'Sejarah, geografi, dan kehidupan sosial.');
                 INSERT INTO courses(title,description)
                   SELECT * FROM (VALUES
                     ('AI - PRODUCT','Create product visuals with AI.'),
                     ('AI - INTERIOR','Create interior designs with AI.'),
                     ('AI - CHARACTER','Create characters with AI.'),
                     ('INTERMEDIATE 2 - INTERIOR','Advanced interior design: materials, lighting and rendering.'),
                     ('INTERMEDIATE 1 - INTERIOR','Interior design basics: space, layout and furniture.'),
                     ('INTERMEDIATE 2 - CHARACTER','Advanced character design: pose, expression and detail.'),
                     ('INTERMEDIATE 1 - CHARACTER','Character design basics: shape, proportion and anatomy.'),
                     ('BASIC 2','Beyond the basics: composition, color and technique.'),
                     ('BASIC 1','Introduction to the tools and design fundamentals.'))
                   WHERE NOT EXISTS (SELECT 1 FROM courses);
                 UPDATE courses SET description='Create product visuals with AI.' WHERE description='Membuat visual produk dengan bantuan AI.';
                 UPDATE courses SET description='Create interior designs with AI.' WHERE description='Membuat desain interior dengan bantuan AI.';
                 UPDATE courses SET description='Create characters with AI.' WHERE description='Membuat karakter dengan bantuan AI.';
                 UPDATE courses SET description='Advanced interior design: materials, lighting and rendering.' WHERE description='Desain interior lanjutan: material, cahaya, dan render.';
                 UPDATE courses SET description='Interior design basics: space, layout and furniture.' WHERE description='Dasar desain interior: ruang, layout, dan furnitur.';
                 UPDATE courses SET description='Advanced character design: pose, expression and detail.' WHERE description='Desain karakter lanjutan: pose, ekspresi, dan detail.';
                 UPDATE courses SET description='Character design basics: shape, proportion and anatomy.' WHERE description='Dasar desain karakter: bentuk, proporsi, dan anatomi.';
                 UPDATE courses SET description='Beyond the basics: composition, color and technique.' WHERE description='Lanjutan dasar: komposisi, warna, dan teknik.';
                 UPDATE courses SET description='Introduction to the tools and design fundamentals.' WHERE description='Pengenalan alat dan dasar-dasar desain.';",
            )?;
            let db: Db = Arc::new(Mutex::new(c));
            let shared = db.clone();
            std::thread::spawn(move || serve(shared));
            app.manage(db);
            if let Some(root) = videos_root() {
                app.asset_protocol_scope().allow_directory(root, true).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            needs_setup, setup_teacher, login, list_students, list_staff, add_student, delete_student,
            list_courses, add_course, delete_course, update_course, set_view, list_lessons, read_sub, server_info, quit
        ])
        .run(tauri::generate_context!())
        .expect("gagal menjalankan aplikasi");
}
