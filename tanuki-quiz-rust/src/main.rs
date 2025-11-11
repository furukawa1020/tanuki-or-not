use axum::{routing::get, routing::post, Json, Router, extract::Path, response::IntoResponse};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::env;
use image::{ImageBuffer, Rgba, DynamicImage, ImageOutputFormat};
use std::io::Cursor;
use bytes::Bytes;
use axum::http::header;
use tokio::fs;

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

#[derive(Serialize)]
struct GeneratedChoice {
    id: usize,
    image_url: String,
    category: String,
}

#[derive(Serialize)]
struct GeneratedQuiz {
    question: String,
    choices: Vec<GeneratedChoice>,
    answer_category: String,
}

async fn generate_quiz() -> Json<GeneratedQuiz> {
    // ensure public/fetched exists
    let mut fetched_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    fetched_dir.push("public");
    fetched_dir.push("fetched");
    let _ = fs::create_dir_all(&fetched_dir).await;

    let mut choices: Vec<GeneratedChoice> = Vec::new();

    // Define categories (these are used as keys to generate images)
    let categories = vec!["forest", "water", "urban", "animal"];

    // For each category, generate one image using the internal generator and save it under public/fetched/{category}
    for (i, cat_key) in categories.iter().enumerate() {
        let key = format!("{}{}", cat_key, i + 1);
        let bytes = match generate_image_bytes(&key) {
            Ok(b) => b,
            Err(_) => continue,
        };

        // save under public/fetched/{category}
        let mut out_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        out_dir.push("public");
        out_dir.push("fetched");
        out_dir.push(cat_key);
        let _ = fs::create_dir_all(&out_dir).await;
        let filename = format!("img_{}.png", i + 1);
        let mut out_path = out_dir.clone();
        out_path.push(&filename);
        let _ = fs::write(&out_path, &bytes).await;

        // make URL relative to `public/` so ServeDir serves it at `/fetched/...`
        let public_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join("public");
        let rel_path = if let Ok(rp) = out_path.strip_prefix(&public_dir) { rp.to_path_buf() } else { out_path.clone() };
        // normalize Windows backslashes to URL slashes
        let image_url = format!("/{}", rel_path.to_string_lossy().replace("\\","/"));

        choices.push(GeneratedChoice { id: i + 1, image_url, category: cat_key.to_string() });
    }

    // Shuffle the choices so order isn't predictable
    let mut rng = rand::thread_rng();
    choices.shuffle(&mut rng);

    // Pick a target category randomly from one of the generated choices
    let target_cat = if let Some(c) = choices.choose(&mut rng) { c.category.clone() } else { "other".to_string() };
    let label = match target_cat.as_str() {
        "forest" => "森",
        "water" => "海/空",
        "urban" => "街並み/都市",
        "animal" => "動物",
        _ => "特徴のある画像",
    };

    let question = format!("次の画像のうち、{} はどれですか？", label);
    Json(GeneratedQuiz { question, choices, answer_category: target_cat })
}

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
        .nest_service("/", ServeDir::new(static_dir));

    let addr: SocketAddr = "0.0.0.0:3000".parse().unwrap();
    println!("listening on http://{}", addr);

    // Use axum's serve helper with a TcpListener
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("listening on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}