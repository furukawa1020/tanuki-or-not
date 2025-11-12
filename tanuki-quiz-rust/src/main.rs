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

// For generated-quiz submissions
#[derive(Deserialize)]
struct GeneratedSubmit {
    selected_category: String,
    answer_category: String,
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
    // Use free image source URLs (no download). We'll return external URLs that the client can load directly.
    let mut choices: Vec<GeneratedChoice> = Vec::new();

    // categories and search keywords
    // Prefer species-specific searches so choices can include たぬき/アナグマ/ハクビシン where possible.
    // We keep a generic 'animal' fallback as a catch-all.
    let categories = vec![
        ("tanuki", "tanuki,raccoon dog,狸"),
        ("anaguma", "badger,anaguma,アナグマ"),
        ("hakubishin", "masked palm civet,hakubishin,ハクビシン"),
        ("animal", "animal,wildlife"),
    ];

    let mut rng = rand::thread_rng();

    for (i, (cat_key, keywords)) in categories.iter().enumerate() {
        // Return external Unsplash Source URLs so the browser loads images directly.
        let mut rng = rand::thread_rng();
        let sig: u64 = rng.gen();
        let image_url = format!("https://source.unsplash.com/800x600/?{}&sig={}", keywords, sig);
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
    Json(GeneratedQuiz { question, choices, answer_category: target_cat })
}

// proxy handler removed to avoid heavy dependencies; the client will load external Unsplash URLs directly

async fn serve_image(Path(name): Path<String>) -> impl IntoResponse {
    // name could be like "tanuki1.png"; strip extension if present
    let key = name.split('.').next().unwrap_or(&name).to_string();
    match generate_image_bytes(&key) {
        Ok(bytes) => {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("image/png"));
                // If upstream didn't return an image content-type, fall back to a generated PNG
                let mut is_image = false;
                if let Some(ct) = ct_hdr {
                    if let Ok(ct_str) = ct.to_str() {
                        if ct_str.starts_with("image/") {
                            if let Ok(hv) = axum::http::HeaderValue::from_str(ct_str) {
                                headers.insert(axum::http::header::CONTENT_TYPE, hv);
                            }
                            is_image = true;
                        }
                    }
                }
                if !is_image {
                    // upstream returned HTML or error page; generate a local PNG fallback
                    if let Ok(png) = generate_image_bytes(key.as_str()) {
                        let mut headers = axum::http::HeaderMap::new();
                        headers.insert(axum::http::header::CONTENT_TYPE, axum::http::HeaderValue::from_static("image/png"));
                        return (axum::http::StatusCode::OK, headers, Bytes::from(png)).into_response();
                    } else {
                        return (axum::http::StatusCode::BAD_GATEWAY).into_response();
                    }
                }
                if !headers.contains_key(axum::http::header::CONTENT_TYPE) {
                    headers.insert(axum::http::header::CONTENT_TYPE, axum::http::HeaderValue::from_static("image/jpeg"));
                }
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
    let correct = payload.selected_category == payload.answer_category;
    Json(QuizResult {
        correct,
        correct_answer: payload.answer_category.clone(),
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
        .route("/api/submit_generated", post(submit_generated))
        .nest_service("/", ServeDir::new(static_dir));

    let addr: SocketAddr = env::var("HOST_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string()).parse().unwrap();
    println!("listening on http://{}", addr);

    // Use axum's serve helper with a TcpListener
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}