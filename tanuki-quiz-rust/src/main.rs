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
use reqwest::Client;
use tokio::fs;
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
}

async fn generate_quiz() -> Json<GeneratedQuiz> {
    // ensure public/fetched exists
    let mut fetched_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    fetched_dir.push("public");
    fetched_dir.push("fetched");
    let _ = fs::create_dir_all(&fetched_dir).await;

    let client = Client::new();
    let mut choices: Vec<GeneratedChoice> = Vec::new();
    let mut rng = rand::thread_rng();

    // fetch 4 images
    for i in 0..4usize {
        let seed: u64 = rng.gen();
        let url = format!("https://picsum.photos/seed/{}/800/600", seed);
        let resp = client.get(&url).send().await;
        if resp.is_err() {
            continue;
        }
        let bytes = match resp.unwrap().bytes().await {
            Ok(b) => b,
            Err(_) => continue,
        };

        let filename = format!("fetched/img_{}.jpg", seed);
        let mut out_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        out_path.push("public");
        out_path.push(&filename);
        // write file
        let _ = fs::write(&out_path, &bytes).await;

        // analyze image: average color
        let dyn = match image::load_from_memory(&bytes) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let thumb = dyn.thumbnail(80, 60);
        let (w, h) = thumb.dimensions();
        let mut r_sum: u64 = 0;
        let mut g_sum: u64 = 0;
        let mut b_sum: u64 = 0;
        let mut cnt: u64 = 0;
        for px in thumb.pixels() {
            let p = px.2.to_rgb();
            r_sum += p[0] as u64;
            g_sum += p[1] as u64;
            b_sum += p[2] as u64;
            cnt += 1;
        }
        if cnt == 0 { continue; }
        let r_avg = (r_sum / cnt) as u8;
        let g_avg = (g_sum / cnt) as u8;
        let b_avg = (b_sum / cnt) as u8;

        let category = if g_avg > r_avg && g_avg > b_avg && g_avg > 100 {
            "forest"
        } else if b_avg > r_avg && b_avg > g_avg && b_avg > 100 {
            "water"
        } else if r_avg > g_avg && r_avg > b_avg && r_avg > 100 {
            "warm"
        } else {
            "other"
        };

        let id = i + 1;
        let image_url = format!("/{}", filename.replace("\\\\","/"));
        choices.push(GeneratedChoice { id, image_url, category: category.to_string() });
    }

    // pick target category (prefer non-other)
    let mut target_cat = "other".to_string();
    for c in &choices {
        if c.category != "other" {
            target_cat = c.category.clone();
            break;
        }
    }
    if target_cat == "other" && !choices.is_empty() {
        target_cat = choices[0].category.clone();
    }

    let label = match target_cat.as_str() {
        "forest" => "森",
        "water" => "海/空",
        "warm" => "赤みのある景色",
        _ => "特徴のある画像",
    };

    let question = format!("次の画像のうち、{} はどれですか？", label);
    Json(GeneratedQuiz { question, choices })
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