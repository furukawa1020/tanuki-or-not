use axum::{routing::get, routing::post, Json, Router, extract::Path, response::IntoResponse};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
// proxy removed: we no longer fetch Unsplash from server side to keep builds light
use tower_http::services::ServeDir;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::env;
use image::{ImageBuffer, Rgba, DynamicImage, ImageOutputFormat};
use std::io::Cursor;
use bytes::Bytes;
use axum::http::header;
use rand::Rng;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use axum::extract::Multipart;
use axum::extract::Query;
use std::collections::HashMap as StdHashMap;
// image::imageops::FilterType not needed currently
use std::fs::File;
use std::io::Write;
use std::io::Read;

#[derive(Serialize, Clone)]
struct QuizQuestion {
    id: usize,
    image_url: String,
    answer: String,
}

#[derive(Deserialize)]
struct QuizAnswer {
    id: usize,
    answer: String,
}

#[derive(Serialize)]
struct QuizResult {
    correct: bool,
    correct_answer: String,
}

// For generated-quiz submissions (client -> server)
#[derive(Deserialize)]
struct GeneratedSubmit {
    quiz_id: String,
    selected_category: String,
}

// In-memory store for active generated quizzes
static QUIZ_STORE: Lazy<Mutex<HashMap<String, (GeneratedQuiz, Instant)>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn get_all_questions() -> Vec<QuizQuestion> {
    vec![
        // たぬき (Tanuki)
    QuizQuestion { id: 1, image_url: "/images/tanuki1.png".to_string(), answer: "たぬき".to_string() },
    QuizQuestion { id: 2, image_url: "/images/tanuki2.png".to_string(), answer: "たぬき".to_string() },
    QuizQuestion { id: 3, image_url: "/images/tanuki3.png".to_string(), answer: "たぬき".to_string() },
        // アナグマ (Anaguma)
    QuizQuestion { id: 4, image_url: "/images/anaguma1.png".to_string(), answer: "アナグマ".to_string() },
    QuizQuestion { id: 5, image_url: "/images/anaguma2.png".to_string(), answer: "アナグマ".to_string() },
    QuizQuestion { id: 6, image_url: "/images/anaguma3.png".to_string(), answer: "アナグマ".to_string() },
        // ハクビシン (Hakubishin)
    QuizQuestion { id: 7, image_url: "/images/hakubishin1.png".to_string(), answer: "ハクビシン".to_string() },
    QuizQuestion { id: 8, image_url: "/images/hakubishin2.png".to_string(), answer: "ハクビシン".to_string() },
    QuizQuestion { id: 9, image_url: "/images/hakubishin3.png".to_string(), answer: "ハクビシン".to_string() },
        // 追加問題 (Mixed)
        QuizQuestion { id: 10, image_url: "/images/tanuki4.png".to_string(), answer: "たぬき".to_string() },
        QuizQuestion { id: 11, image_url: "/images/anaguma4.png".to_string(), answer: "アナグマ".to_string() },
        QuizQuestion { id: 12, image_url: "/images/hakubishin4.png".to_string(), answer: "ハクビシン".to_string() },
    ]
}

#[derive(Serialize, Clone)]
struct GeneratedChoice {
    id: usize,
    image_url: String,
    category: String,
}

#[derive(Serialize, Clone)]
struct GeneratedQuiz {
    question: String,
    choices: Vec<GeneratedChoice>,
    answer_category: String,
}

// Response returned to client when creating a quiz (no answer included)
#[derive(Serialize)]
struct GeneratedQuizResponse {
    id: String,
    question: String,
    choices: Vec<GeneratedChoice>,
}

#[derive(Deserialize)]
struct AdminUploadJson {
    filename: String,
    b64: String,
}

#[derive(Serialize)]
struct AdminUploadResult {
    ok: bool,
    saved_filename: Option<String>,
    thumb_filename: Option<String>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct AssetIndexEntry {
    filename: String,
    size: u64,
    thumb: bool,
    phash: Option<String>,
    uploaded_at: String,
}

fn index_path() -> PathBuf { PathBuf::from("public").join("assets").join("index.json") }

fn load_index() -> Vec<AssetIndexEntry> {
    let p = index_path();
    if !p.exists() { return vec![]; }
    let mut s = String::new();
    if let Ok(mut f) = File::open(&p) {
        if f.read_to_string(&mut s).is_ok() {
            if let Ok(v) = serde_json::from_str::<Vec<AssetIndexEntry>>(&s) { return v; }
        }
    }
    vec![]
}

fn save_index(entries: &Vec<AssetIndexEntry>) {
    let p = index_path();
    if let Ok(mut f) = File::create(&p) {
        let _ = f.write_all(serde_json::to_string_pretty(entries).unwrap_or_else(|_| "[]".to_string()).as_bytes());
    }
}

fn compute_ahash(img: &DynamicImage) -> String {
    // average hash (8x8 -> 64 bits)
    let small = img.resize_exact(8, 8, image::imageops::FilterType::Nearest).to_luma8();
    let mut sum: u32 = 0;
    for p in small.pixels() { sum += p[0] as u32; }
    let avg = (sum / 64) as u8;
    let mut bits: u64 = 0;
    for (i, p) in small.pixels().enumerate() {
        if p[0] >= avg { bits |= 1u64 << i; }
    }
    format!("{:016x}", bits)
}

fn hamming_hex(a_hex: &str, b_hex: &str) -> Option<u32> {
    if a_hex.len() != 16 || b_hex.len() != 16 { return None; }
    let a = u64::from_str_radix(a_hex, 16).ok()?;
    let b = u64::from_str_radix(b_hex, 16).ok()?;
    Some((a ^ b).count_ones())
}

// similar search endpoint: ?filename=<name>&token=<token>&max_hamming=10
async fn admin_similar(Query(q): Query<StdHashMap<String, String>>) -> Json<Vec<AdminListEntry>> {
    let token = q.get("token").cloned().unwrap_or_default();
    if !check_admin_token_token(&token) { return Json(vec![]); }
    let filename = match q.get("filename") { Some(s) => s.clone(), None => return Json(vec![]) };
    let max_hamming: u32 = q.get("max_hamming").and_then(|s| s.parse().ok()).unwrap_or(10);
    let idx = load_index();
    let base = match idx.iter().find(|e| e.filename == filename) { Some(e) => e.phash.clone().unwrap_or_default(), None => return Json(vec![]) };
    let mut out = Vec::new();
    for e in idx.iter() {
        if e.filename == filename { continue; }
        if let (Some(a), Some(b)) = (Some(base.as_str()), e.phash.as_deref()) {
            if let Some(dist) = hamming_hex(a, b) {
                if dist <= max_hamming { out.push(AdminListEntry { filename: e.filename.clone(), size: e.size, thumb: e.thumb }); }
            }
        }
    }
    Json(out)
}

async fn generate_quiz() -> Json<GeneratedQuizResponse> {
    // Use free image source URLs (no download). We'll return external URLs that the client can load directly.
    let mut choices: Vec<GeneratedChoice> = Vec::new();
    // categories and preferred local filenames (user should place real photos here)
    let categories = vec!["tanuki", "anaguma", "hakubishin"];
    // For each category, prefer local files public/assets/<category><n>.jpg (1..3)
    let static_dir = PathBuf::from("public").join("assets");
    let mut rng = rand::thread_rng();
    for (i, cat_key) in categories.iter().enumerate() {
        let mut image_url = String::new();
        // look for any matching local files in public/assets (support any number)
        if static_dir.exists() {
            let mut candidates: Vec<String> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&static_dir) {
                for entry in entries.flatten() {
                    if let Some(name_os) = entry.file_name().to_str() {
                        let name = name_os.to_string();
                        let lower = name.to_lowercase();
                        if lower.starts_with(&cat_key.to_string()) && (lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".png")) {
                            candidates.push(name);
                        }
                    }
                }
            }
            if !candidates.is_empty() {
                let picked = candidates.choose(&mut rng).unwrap().clone();
                // prefer thumbnail if exists
                let thumb_path = PathBuf::from("public").join("assets").join("thumbs").join(&picked);
                if thumb_path.exists() {
                    image_url = format!("/assets/thumbs/{}", picked);
                } else {
                    image_url = format!("/assets/{}", picked);
                }
            }
        }

        // if no local file, fallback to Unsplash Source
        if image_url.is_empty() {
            let keywords = match *cat_key {
                "tanuki" => "tanuki,raccoon dog,狸",
                "anaguma" => "badger,anaguma,アナグマ",
                "hakubishin" => "masked palm civet,hakubishin,ハクビシン",
                _ => "animal,wildlife",
            };
            let sig: u64 = rng.gen();
            image_url = format!("https://source.unsplash.com/800x600/?{}&sig={}", keywords, sig);
        }

        choices.push(GeneratedChoice { id: i + 1, image_url, category: cat_key.to_string() });
    }

    // Shuffle so order isn't predictable
    choices.shuffle(&mut rng);

    // select target category
    let target_cat = if let Some(c) = choices.choose(&mut rng) { c.category.clone() } else { "other".to_string() };
    let label = match target_cat.as_str() {
        "tanuki" => "たぬき",
        "anaguma" => "アナグマ",
        "hakubishin" => "ハクビシン",
        "animal" => "動物",
        _ => "特徴のある画像",
    };

    let question = format!("次の画像のうち、{} はどれですか？", label);
    let quiz = GeneratedQuiz { question: question.clone(), choices: choices.clone(), answer_category: target_cat.clone() };

    // generate id and store it
    let id = Uuid::new_v4().to_string();
    QUIZ_STORE.lock().insert(id.clone(), (quiz, Instant::now()));

    Json(GeneratedQuizResponse { id, question, choices })
}

// simple admin upload via JSON { filename, b64 }
async fn admin_upload_json(Json(payload): Json<AdminUploadJson>) -> Json<AdminUploadResult> {
    // sanitize filename: keep ascii alnum, dash, underscore and extension
    let name = payload.filename.clone();
    if name.contains('/') || name.contains('\\') {
        return Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some("invalid filename".to_string()) });
    }
    // lower-case extension
    let safe: String = name.chars().map(|c| {
        if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' }
    }).collect();
    let safe = safe;

    // decode base64
    let data = match BASE64.decode(payload.b64.trim()) {
        Ok(d) => d,
        Err(e) => return Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some(format!("base64 decode error: {}", e)) }),
    };

    // verify image
    let img_dyn = match image::load_from_memory(&data) {
        Ok(d) => d,
        Err(e) => return Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some(format!("invalid image data: {}", e)) }),
    };

    // ensure dirs
    let assets_dir = PathBuf::from("public").join("assets");
    let thumbs_dir = assets_dir.join("thumbs");
    if let Err(e) = std::fs::create_dir_all(&thumbs_dir) { return Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some(format!("mkdir error: {}", e)) }); }

    // ensure unique filename if exists
    let mut target = assets_dir.join(&safe);
    let mut counter = 1;
    while target.exists() {
    // insert suffix before extension
    let tmp = PathBuf::from(&safe);
    let stem = tmp.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = tmp.extension().and_then(|s| s.to_str()).unwrap_or("");
        let newname = if ext.is_empty() { format!("{}-{}", stem, counter) } else { format!("{}-{}.{}", stem, counter, ext) };
        target = assets_dir.join(&newname);
        counter += 1;
    }

    // write original
    match File::create(&target).and_then(|mut f| f.write_all(&data)) {
        Ok(_) => {},
        Err(e) => return Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some(format!("write error: {}", e)) }),
    }

    // create thumbnail 320x240 (maintain aspect via thumbnail method)
    let thumb = img_dyn.thumbnail(320, 240).to_rgba8();
    let thumb_path = thumbs_dir.join(target.file_name().and_then(|s| s.to_str()).unwrap_or("thumb.png"));
    if let Err(e) = thumb.save(&thumb_path) {
        return Json(AdminUploadResult { ok: false, saved_filename: target.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()), thumb_filename: None, message: Some(format!("thumbnail save error: {}", e)) });
    }

    // compute phash and update index
    let phash = compute_ahash(&img_dyn);
    let size = target.metadata().map(|m| m.len()).unwrap_or(0);
    let uploaded_at = chrono::Utc::now().to_rfc3339();
    let mut idx = load_index();
    idx.retain(|e| e.filename != target.file_name().and_then(|s| s.to_str()).unwrap_or(""));
    idx.push(AssetIndexEntry { filename: target.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string(), size, thumb: true, phash: Some(phash.clone()), uploaded_at });
    save_index(&idx);

    Json(AdminUploadResult { ok: true, saved_filename: target.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()), thumb_filename: thumb_path.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()), message: None })
}

// multipart upload handler (form submit)
async fn admin_upload_multipart(Query(q): Query<StdHashMap<String, String>>, mut multipart: Multipart) -> Json<AdminUploadResult> {
    // auth via query ?token=
    let token = q.get("token").cloned().unwrap_or_default();
    if !check_admin_token_token(&token) {
        return Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some("unauthorized".to_string()) });
    }

    // find first file field
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
    let _name = field.name().map(|s| s.to_string()).unwrap_or_else(|| "file".to_string());
        if field.file_name().is_none() { continue; }
        let filename = field.file_name().unwrap().to_string();
        // sanitize
        if filename.contains('/') || filename.contains('\\') { continue; }

        let safe: String = filename.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' }).collect();

        // read bytes
        let data = match field.bytes().await {
            Ok(d) => d.to_vec(),
            Err(e) => return Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some(format!("read field error: {}", e)) }),
        };

        // validate image
        let img_dyn = match image::load_from_memory(&data) {
            Ok(d) => d,
            Err(e) => return Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some(format!("invalid image data: {}", e)) }),
        };

        let assets_dir = PathBuf::from("public").join("assets");
        let thumbs_dir = assets_dir.join("thumbs");
        if let Err(e) = std::fs::create_dir_all(&thumbs_dir) { return Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some(format!("mkdir error: {}", e)) }); }

        let mut target = assets_dir.join(&safe);
        let mut counter = 1;
        while target.exists() {
            let tmp = PathBuf::from(&safe);
            let stem = tmp.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let ext = tmp.extension().and_then(|s| s.to_str()).unwrap_or("");
            let newname = if ext.is_empty() { format!("{}-{}", stem, counter) } else { format!("{}-{}.{}", stem, counter, ext) };
            target = assets_dir.join(&newname);
            counter += 1;
        }

        if let Err(e) = std::fs::write(&target, &data) { return Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some(format!("write error: {}", e)) }); }

    let thumb = img_dyn.thumbnail(320, 240).to_rgba8();
    let thumb_path = thumbs_dir.join(target.file_name().and_then(|s| s.to_str()).unwrap_or("thumb.png"));
    if let Err(e) = thumb.save(&thumb_path) { return Json(AdminUploadResult { ok: false, saved_filename: target.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()), thumb_filename: None, message: Some(format!("thumbnail save error: {}", e)) }); }

    // compute phash and update index
    let phash = compute_ahash(&img_dyn);
    let size = target.metadata().map(|m| m.len()).unwrap_or(0);
    let uploaded_at = chrono::Utc::now().to_rfc3339();
    let mut idx = load_index();
    idx.retain(|e| e.filename != target.file_name().and_then(|s| s.to_str()).unwrap_or(""));
    idx.push(AssetIndexEntry { filename: target.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string(), size, thumb: true, phash: Some(phash.clone()), uploaded_at });
    save_index(&idx);

    return Json(AdminUploadResult { ok: true, saved_filename: target.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()), thumb_filename: thumb_path.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()), message: None });
    }

    Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some("no file field found".to_string()) })
}

fn check_admin_token_token(token: &str) -> bool {
    let expected = env::var("ADMIN_TOKEN").unwrap_or_else(|_| "admin-token".to_string());
    token == expected
}

// proxy handler removed to avoid heavy dependencies; the client will load external Unsplash URLs directly

async fn serve_image(Path(name): Path<String>) -> impl IntoResponse {
    // name could be like "tanuki1.png"; strip extension if present
    let key = name.split('.').next().unwrap_or(&name).to_string();
    match generate_image_bytes(&key) {
        Ok(bytes) => {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("image/png"));
            (axum::http::StatusCode::OK, headers, Bytes::from(bytes)).into_response()
        }
        Err(_) => (axum::http::StatusCode::NOT_FOUND).into_response(),
    }
}

fn generate_image_bytes(key: &str) -> Result<Vec<u8>, ()> {
    // 800x600 image
    let width = 800u32;
    let height = 600u32;
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);

    // Background and primary face color based on key prefix
    let (bg, face) = if key.starts_with("tanuki") {
        (Rgba([0xFF, 0xD7, 0x00, 0xFF]), Rgba([0xFF, 0xF5, 0xE6, 0xFF])) // gold bg
    } else if key.starts_with("anaguma") {
        (Rgba([0xA9, 0xA9, 0xA9, 0xFF]), Rgba([0xF5, 0xF5, 0xF5, 0xFF])) // gray
    } else if key.starts_with("hakubishin") {
        (Rgba([0x8B, 0x45, 0x13, 0xFF]), Rgba([0xFF, 0xFF, 0xFF, 0xFF])) // brown
    } else {
        (Rgba([0xDD, 0xDD, 0xDD, 0xFF]), Rgba([0xFF, 0xFF, 0xFF, 0xFF]))
    };

    // fill background
    for pixel in img.pixels_mut() {
        *pixel = bg;
    }

    // draw a face circle in center
    let cx = (width / 2) as i32;
    let cy = (height / 2) as i32;
    let r = (height.min(width) / 3) as i32;
    draw_filled_circle(&mut img, cx, cy, r, face);

    // eyes
    let eye = Rgba([0, 0, 0, 0xFF]);
    draw_filled_circle(&mut img, cx - r/3, cy - r/6, r/10, eye);
    draw_filled_circle(&mut img, cx + r/3, cy - r/6, r/10, eye);

    // species-specific mark
    if key.starts_with("hakubishin") {
        // a mask stripe across face
        let stripe = Rgba([0x80, 0x80, 0x80, 0xFF]);
        for y in (cy - r/6)..=(cy + r/8) {
            for x in (cx - r/2)..=(cx + r/2) {
                if in_circle(cx, cy, r, x, y) {
                    let p = img.get_pixel_mut(x as u32, y as u32);
                    *p = stripe;
                }
            }
        }
    }

    // render to PNG bytes
    let dyn_img = DynamicImage::ImageRgba8(img);
    let mut buf: Vec<u8> = Vec::new();
    dyn_img.write_to(&mut Cursor::new(&mut buf), ImageOutputFormat::Png).map_err(|_| ())?;
    Ok(buf)
}

fn in_circle(cx: i32, cy: i32, r: i32, x: i32, y: i32) -> bool {
    let dx = x - cx;
    let dy = y - cy;
    dx*dx + dy*dy <= r*r
}

fn draw_filled_circle(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, cx: i32, cy: i32, r: i32, color: Rgba<u8>) {
    let (width, height) = (img.width() as i32, img.height() as i32);
    let x0 = (cx - r).max(0);
    let x1 = (cx + r).min(width-1);
    let y0 = (cy - r).max(0);
    let y1 = (cy + r).min(height-1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            if in_circle(cx, cy, r, x, y) {
                let p = img.get_pixel_mut(x as u32, y as u32);
                *p = color;
            }
        }
    }
}

async fn get_quiz_question() -> Json<QuizQuestion> {
    let questions = get_all_questions();
    let question = questions.choose(&mut rand::thread_rng()).unwrap();
    Json(question.clone())
}

async fn submit_answer(Json(payload): Json<QuizAnswer>) -> Json<QuizResult> {
    let questions = get_all_questions();
    let question = questions.iter().find(|q| q.id == payload.id).unwrap();
    let correct = question.answer == payload.answer;
    Json(QuizResult {
        correct,
        correct_answer: question.answer.clone(),
    })
}

async fn submit_generated(Json(payload): Json<GeneratedSubmit>) -> Json<QuizResult> {
    // lookup quiz by id
    let mut store = QUIZ_STORE.lock();
    if let Some((stored_quiz, _)) = store.remove(&payload.quiz_id) {
        let correct = payload.selected_category == stored_quiz.answer_category;
        Json(QuizResult {
            correct,
            correct_answer: stored_quiz.answer_category.clone(),
        })
    } else {
        // missing or expired quiz — treat as incorrect but provide a generic response
        Json(QuizResult {
            correct: false,
            correct_answer: "unknown".to_string(),
        })
    }
}

#[derive(Serialize)]
struct AdminListEntry {
    filename: String,
    size: u64,
    thumb: bool,
}

async fn admin_list(Query(q): Query<StdHashMap<String, String>>) -> Json<Vec<AdminListEntry>> {
    let token = q.get("token").cloned().unwrap_or_default();
    if !check_admin_token_token(&token) { return Json(vec![]); }
    let assets_dir = PathBuf::from("public").join("assets");
    let mut out = Vec::new();
    // enrich with index.json if present
    let index = load_index();
    if !index.is_empty() {
        for e in index {
            out.push(AdminListEntry { filename: e.filename.clone(), size: e.size, thumb: e.thumb });
        }
    } else {
        if let Ok(entries) = std::fs::read_dir(&assets_dir) {
            for e in entries.flatten() {
                if let Ok(mt) = e.metadata() {
                    if let Some(name) = e.file_name().to_str() {
                        // skip thumbs directory
                        if name == "thumbs" { continue; }
                        out.push(AdminListEntry { filename: name.to_string(), size: mt.len(), thumb: PathBuf::from("public").join("assets").join("thumbs").join(name).exists() });
                    }
                }
            }
        }
    }
    Json(out)
}

#[derive(Deserialize)]
struct AdminDeleteReq { filename: String }

async fn admin_delete(Query(q): Query<StdHashMap<String, String>>, Json(payload): Json<AdminDeleteReq>) -> Json<AdminUploadResult> {
    let token = q.get("token").cloned().unwrap_or_default();
    if !check_admin_token_token(&token) { return Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some("unauthorized".to_string()) }); }
    let assets_dir = PathBuf::from("public").join("assets");
    let target = assets_dir.join(&payload.filename);
    let thumb = assets_dir.join("thumbs").join(&payload.filename);
    let mut ok = false;
    if target.exists() { let _ = std::fs::remove_file(&target); ok = true; }
    if thumb.exists() { let _ = std::fs::remove_file(&thumb); }
    if ok {
        Json(AdminUploadResult { ok: true, saved_filename: Some(payload.filename), thumb_filename: None, message: None })
    } else {
        Json(AdminUploadResult { ok: false, saved_filename: None, thumb_filename: None, message: Some("not found".to_string()) })
    }
}

#[tokio::main]
async fn main() {
    // Build absolute path to `public` so the server works regardless of CWD
    let mut static_dir: PathBuf = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    static_dir.push("public");

    // API routes registered first, then serve static files as the fallback.
    let app = Router::new()
        .route("/api/quiz", get(get_quiz_question))
        .route("/api/generate_quiz", get(generate_quiz))
    .route("/images/:name", get(serve_image))
        .route("/api/submit", post(submit_answer))
        .route("/api/submit_generated", post(submit_generated))
        .route("/api/admin/upload", post(admin_upload_json))
        .route("/api/admin/upload_multipart", post(admin_upload_multipart))
        .route("/api/admin/list", get(admin_list))
    .route("/api/admin/similar", get(admin_similar))
        .route("/api/admin/delete", post(admin_delete))
        .nest_service("/", ServeDir::new(static_dir));

    let addr: SocketAddr = env::var("HOST_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string()).parse().unwrap();
    println!("listening on http://{}", addr);

    // Use axum's serve helper with a TcpListener
    // spawn a background cleanup task to remove old quizzes
    let _cleanup_handle = tokio::spawn(async move {
        let ttl = Duration::from_secs(60 * 5); // 5 minutes
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let now = Instant::now();
            let mut store = QUIZ_STORE.lock();
            let keys_to_remove: Vec<String> = store.iter()
                .filter_map(|(k, (_v, ts))| if now.duration_since(*ts) > ttl { Some(k.clone()) } else { None })
                .collect();
            for k in keys_to_remove {
                store.remove(&k);
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}