use axum::{routing::get, Json, Router};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

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

// NOTE: Using placeholders for images.
// In a real application, you would have a list of actual images.
fn get_all_questions() -> Vec<QuizQuestion> {
    vec![
        QuizQuestion {
            id: 1,
            image_url: "https://placehold.jp/3d4070/ffffff/400x300.png?text=たぬき%20(Tanuki)".to_string(),
            answer: "たぬき".to_string(),
        },
        QuizQuestion {
            id: 2,
            image_url: "https://placehold.jp/703d40/ffffff/400x300.png?text=アナグマ%20(Anaguma)".to_string(),
            answer: "アナグマ".to_string(),
        },
        QuizQuestion {
            id: 3,
            image_url: "https://placehold.jp/40703d/ffffff/400x300.png?text=ハクビシン%20(Hakubishin)".to_string(),
            answer: "ハクビシン".to_string(),
        },
        QuizQuestion {
            id: 4,
            image_url: "https://placehold.jp/3d4070/ffffff/400x300.png?text=たぬき%20(別ポーズ)".to_string(),
            answer: "たぬき".to_string(),
        },
        QuizQuestion {
            id: 5,
            image_url: "https://placehold.jp/703d40/ffffff/400x300.png?text=アナグマ%20(別ポーズ)".to_string(),
            answer: "アナグマ".to_string(),
        },
        QuizQuestion {
            id: 6,
            image_url: "https://placehold.jp/40703d/ffffff/400x300.png?text=ハクビシン%20(別ポーズ)".to_string(),
            answer: "ハクビシン".to_string(),
        },
        QuizQuestion {
            id: 7,
            image_url: "https://placehold.jp/3d4070/ffffff/400x300.png?text=たぬき%20(夜行性)".to_string(),
            answer: "たぬき".to_string(),
        },
        QuizQuestion {
            id: 8,
            image_url: "https://placehold.jp/703d40/ffffff/400x300.png?text=アナグマ%20(土の中)".to_string(),
            answer: "アナグマ".to_string(),
        },
        QuizQuestion {
            id: 9,
            image_url: "https://placehold.jp/40703d/ffffff/400x300.png?text=ハクビシン%20(木の上)".to_string(),
            answer: "ハクビシン".to_string(),
        },
    ]
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
    let app = Router::new()
        .nest_service("/", ServeDir::new("public"))
        .route("/api/quiz", get(get_quiz_question))
        .route("/api/submit", axum::routing::post(submit_answer));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("listening on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}