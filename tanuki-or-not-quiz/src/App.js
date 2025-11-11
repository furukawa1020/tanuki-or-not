import React, { useState } from 'react';
import './App.css';

const animals = [
  {
    name: 'タヌキ',
    image: 'https://via.placeholder.com/400x300.png?text=Tanuki',
  },
  {
    name: 'アナグマ',
    image: 'https://via.placeholder.com/400x300.png?text=Badger',
  },
  {
    name: 'ハクビシン',
    image: 'https://via.placeholder.com/400x300.png?text=Masked+Palm+Civet',
  },
];

// questions配列を生成
const questions = animals.map((animal) => {
  return {
    image: animal.image,
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
              <img src={questions[currentQuestion].image} alt="animal" />
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
