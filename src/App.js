import React, { useState } from 'react';
import './App.css';

const embeddedImages = {
  tanuki: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='400' height='300'><rect fill='%23dddddd' width='100%25' height='100%25'/><text x='50%25' y='50%25' dominant-baseline='middle' text-anchor='middle' font-size='36' fill='%23000000'>Tanuki</text></svg>",
  anaguma: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='400' height='300'><rect fill='%23dddddd' width='100%25' height='100%25'/><text x='50%25' y='50%25' dominant-baseline='middle' text-anchor='middle' font-size='36' fill='%23000000'>Badger</text></svg>",
  hakubishin: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='400' height='300'><rect fill='%23dddddd' width='100%25' height='100%25'/><text x='50%25' y='50%25' dominant-baseline='middle' text-anchor='middle' font-size='28' fill='%23000000'>Masked%20Palm%20Civet</text></svg>",
};

// Prefer serving local real photos from /assets/*.jpg (place files in public/assets/),
// but fall back to embedded SVGs when no file is found.
const animalDefs = [
  { name: 'タヌキ', key: 'tanuki', count: 5 },
  { name: 'アナグマ', key: 'anaguma', count: 5 },
  { name: 'ハクビシン', key: 'hakubishin', count: 5 },
];

// Build questions with randomized images (called at quiz start / retry)
function buildQuestions() {
  const optionsTemplate = animalDefs.map((a) => ({ answerText: a.name }));
  return animalDefs.map((animal) => {
    const n = Math.floor(Math.random() * animal.count) + 1;
    const imagePath = `/assets/${animal.key}${n}.jpg`;
    return {
      image: imagePath,
      fallbackKey: animal.key,
      options: optionsTemplate.map((o) => ({ ...o, isCorrect: o.answerText === animal.name })),
      correctAnswer: animal.name,
    };
  });
}


function QuestionImage({ question, embeddedImages }) {
  const [src, setSrc] = useState(question.image);
  const [tried, setTried] = useState([]);

  // build candidate list (try the app's /assets, then the rust subproject's public assets,
  // try common extensions, then embedded SVG)
  const makeCandidates = () => {
    const candidates = [];
    if (question.image) candidates.push(question.image);
    // if image looks like '/assets/tanuki1.jpg', extract basename
    try {
      const parts = question.image.split('/');
      const name = parts[parts.length - 1];
      if (name) {
        candidates.push(`/tanuki-quiz-rust/public/assets/${name}`);
        // try common alternate extensions
        const base = name.replace(/\.(jpg|jpeg|png|gif)$/i, '');
        candidates.push(`/tanuki-quiz-rust/public/assets/${base}.jpg`);
        candidates.push(`/tanuki-quiz-rust/public/assets/${base}.png`);
        candidates.push(`/tanuki-quiz-rust/public/assets/${base}.jpeg`);
      }
    } catch (e) {
      // ignore
    }
    // final fallback: embedded svg
    const key = question.fallbackKey || 'tanuki';
    candidates.push(embeddedImages[key]);
    return Array.from(new Set(candidates));
  };

  React.useEffect(() => {
    // reset when question changes
    setSrc(question.image);
    setTried([]);
  }, [question]);

  const candidates = makeCandidates();

  const handleError = () => {
    // mark current src as tried and move to next
    setTried((prev) => {
      const next = [...prev, src];
      // find next candidate not tried
      const nextCandidate = candidates.find((c) => !next.includes(c));
      if (nextCandidate) setSrc(nextCandidate);
      return next;
    });
  };

  return (
    <img src={src} alt="animal" onError={handleError} style={{maxWidth: '100%', maxHeight: '100%', objectFit: 'contain'}} />
  );
}

function App() {
  const [currentQuestion, setCurrentQuestion] = useState(0);
  const [score, setScore] = useState(0);
  const [showScore, setShowScore] = useState(false);
  const [questions, setQuestions] = useState(() => buildQuestions());

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
          <button onClick={() => {
              // regenerate randomized questions and reset state
              setQuestions(buildQuestions());
              setScore(0);
              setCurrentQuestion(0);
              setShowScore(false);
            }} className="retry-button">
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
                <QuestionImage
                  question={questions[currentQuestion]}
                  embeddedImages={embeddedImages}
                  key={currentQuestion}
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
