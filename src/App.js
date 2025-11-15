import React, { useState } from 'react';
import './App.css';

const embeddedImages = {
  tanuki: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='400' height='300'><rect fill='%23dddddd' width='100%25' height='100%25'/><text x='50%25' y='50%25' dominant-baseline='middle' text-anchor='middle' font-size='36' fill='%23000000'>Tanuki</text></svg>",
  anaguma: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='400' height='300'><rect fill='%23dddddd' width='100%25' height='100%25'/><text x='50%25' y='50%25' dominant-baseline='middle' text-anchor='middle' font-size='36' fill='%23000000'>Badger</text></svg>",
  hakubishin: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='400' height='300'><rect fill='%23dddddd' width='100%25' height='100%25'/><text x='50%25' y='50%25' dominant-baseline='middle' text-anchor='middle' font-size='28' fill='%23000000'>Masked%20Palm%20Civet</text></svg>",
};

// Prefer serving local real photos from /assets/*.jpg (place files in public/assets/),
// but fall back to embedded SVGs when no file is found.
const animals = [
  { name: 'タヌキ', imageLocal: '/assets/tanuki1.jpg', fallbackKey: 'tanuki' },
  { name: 'アナグマ', imageLocal: '/assets/anaguma1.jpg', fallbackKey: 'anaguma' },
  { name: 'ハクビシン', imageLocal: '/assets/hakubishin1.jpg', fallbackKey: 'hakubishin' },
];

// questions配列を生成
const questions = animals.map((animal) => {
  return {
    // start with local path; <img onError> will fall back to embedded SVG if missing
    image: animal.imageLocal,
    fallbackKey: animal.fallbackKey,
    options: animals.map((a) => ({ answerText: a.name, isCorrect: a.name === animal.name })),
    correctAnswer: animal.name,
  };
});


function App() {
  const [currentQuestion, setCurrentQuestion] = useState(0);
  const [score, setScore] = useState(0);
  const [showScore, setShowScore] = useState(false);

  const handleAnswerButtonClick = (isCorrect) => {
    if (isCorrect) {
      setScore(score + 1);
    }

    const nextQuestion = currentQuestion + 1;
    if (nextQuestion < questions.length) {
      setCurrentQuestion(nextQuestion);
    } else {
      setShowScore(true);
    }
  };

  const shareOnTwitter = () => {
    const text = `タヌキクイズで${questions.length}問中${score}問正解しました！あなたも見分けられるかな？`;
    const url = window.location.href;
    const hashtags = 'タヌキクイズ,tanukiornot';
    const twitterUrl = `https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}&url=${encodeURIComponent(url)}&hashtags=${encodeURIComponent(hashtags)}`;
    window.open(twitterUrl, '_blank');
  };

  return (
    <div className="app">
      <header className="app-header">
        <h1>タヌキ？ アナグマ？ ハクビシン？</h1>
      </header>
      {showScore ? (
        <div className="score-section">
          <h2>結果</h2>
          <p>{questions.length}問中 {score}問 正解です！</p>
          <button onClick={shareOnTwitter} className="share-button">
            Xで結果をシェア
          </button>
          <button onClick={() => window.location.reload()} className="retry-button">
            もう一度挑戦
          </button>
        </div>
      ) : (
        <>
          <div className="question-section">
            <div className="question-count">
              <span>Question {currentQuestion + 1}</span>/{questions.length}
            </div>
            <div className="question-image">
              <img
                src={questions[currentQuestion].image}
                alt="animal"
                onError={(e) => {
                  const q = questions[currentQuestion];
                  const key = q.fallbackKey || 'tanuki';
                  e.currentTarget.src = embeddedImages[key];
                }}
              />
            </div>
          </div>
          <div className="answer-section">
            <p>この動物はなに？</p>
            {questions[currentQuestion].options.map((option, index) => (
              <button key={index} onClick={() => handleAnswerButtonClick(option.isCorrect)}>
                {option.answerText}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

export default App;
