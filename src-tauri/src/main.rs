#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

const PORT: u16 = 8787;
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
                   ('PB','pb001','','pb'),('Trial','trial','','trial');",
            )?;
            let db: Db = Arc::new(Mutex::new(c));
            let shared = db.clone();
            std::thread::spawn(move || serve(shared));
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            needs_setup, setup_teacher, login, list_students, list_staff, add_student, delete_student,
            list_courses, add_course, delete_course, server_info, quit
        ])
        .run(tauri::generate_context!())
        .expect("gagal menjalankan aplikasi");
}
