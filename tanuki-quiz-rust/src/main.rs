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

fn get_all_questions() -> Vec<QuizQuestion> {
    vec![
        // たぬき (Tanuki)
        QuizQuestion {
            id: 1,
            image_url: "https://placehold.co/400x300/FFD700/000000?text=たぬき%20(正面)".to_string(),
            answer: "たぬき".to_string(),
        },
        QuizQuestion {
            id: 2,
            image_url: "https://placehold.co/400x300/FFD700/000000?text=たぬき%20(横向き)".to_string(),
            answer: "たぬき".to_string(),
        },
        QuizQuestion {
            id: 3,
            image_url: "https://placehold.co/400x300/FFD700/000000?text=たぬき%20(座る)".to_string(),
            answer: "たぬき".to_string(),
        },
        // アナグマ (Anaguma)
        QuizQuestion {
            id: 4,
            image_url: "https://placehold.co/400x300/A9A9A9/000000?text=アナグマ%20(土を掘る)".to_string(),
            answer: "アナグマ".to_string(),
        },
        QuizQuestion {
            id: 5,
            image_url: "https://placehold.co/400x300/A9A9A9/000000?text=アナグマ%20(夜間)".to_string(),
            answer: "アナグマ".to_string(),
        },
        QuizQuestion {
            id: 6,
            image_url: "https://placehold.co/400x300/A9A9A9/000000?text=アナグマ%20(巣穴の近く)".to_string(),
            answer: "アナグマ".to_string(),
        },
        // ハクビシン (Hakubishin)
        QuizQuestion {
            id: 7,
            image_url: "https://placehold.co/400x300/8B4513/FFFFFF?text=ハクビシン%20(木の上)".to_string(),
            answer: "ハクビシン".to_string(),
        },
        QuizQuestion {
            id: 8,
            image_url: "https://placehold.co/400x300/8B4513/FFFFFF?text=ハクビシン%20(顔の模様)".to_string(),
            answer: "ハクビシン".to_string(),
        },
        QuizQuestion {
            id: 9,
            image_url: "https://placehold.co/400x300/8B4513/FFFFFF?text=ハクビシン%20(屋根の上)".to_string(),
            answer: "ハクビシン".to_string(),
        },
        // 追加問題 (Mixed)
        QuizQuestion {
            id: 10,
            image_url: "https://placehold.co/400x300/FFD700/000000?text=たぬき%20(冬毛)".to_string(),
            answer: "たぬき".to_string(),
        },
        QuizQuestion {
            id: 11,
            image_url: "https://placehold.co/400x300/A9A9A9/000000?text=アナグマ%20(親子)".to_string(),
            answer: "アナグマ".to_string(),
        },
        QuizQuestion {
            id: 12,
            image_url: "https://placehold.co/400x300/8B4513/FFFFFF?text=ハクビシン%20(果物を食べる)".to_string(),
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