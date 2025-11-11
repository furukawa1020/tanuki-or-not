
document.addEventListener('DOMContentLoaded', () => {
    const quizImage = document.getElementById('quiz-image');
    const optionsContainer = document.getElementById('options');
    const resultText = document.getElementById('result-message');
    const shareContainer = document.getElementById('share-container');
    const shareButton = document.getElementById('share-button');

    let currentQuestion = null;

    const animals = ["たぬき", "アナグマ", "ハクビシン"];

    async function loadQuestion() {
        try {
            const response = await fetch('/api/quiz');
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            currentQuestion = await response.json();
            
            quizImage.src = currentQuestion.image_url;
            resultText.textContent = '';
            shareContainer.style.display = 'none';
            optionsContainer.innerHTML = '';

            animals.forEach(animal => {
                const button = document.createElement('button');
                button.textContent = animal;
                button.className = 'option-button';
                button.onclick = () => submitAnswer(animal);
                optionsContainer.appendChild(button);
            });

        } catch (error) {
            console.error("Failed to load question:", error);
            console.log("resultText in catch block:", resultText);
            resultText.textContent = "クイズの読み込みに失敗しました。";
        }
    }

    async function submitAnswer(selectedAnswer) {
        if (!currentQuestion) return;

        try {
            const response = await fetch('/api/submit', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    id: currentQuestion.id,
                    answer: selectedAnswer,
                }),
            });
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            const result = await response.json();

            if (result.correct) {
                resultText.textContent = "正解！";
            } else {
                resultText.textContent = `残念！正解は ${result.correct_answer} でした。`;
            }
            
            // Disable buttons after answering
            const buttons = optionsContainer.getElementsByTagName('button');
            for(let button of buttons) {
                button.disabled = true;
            }

            setupShareButton(result.correct);
            shareContainer.style.display = 'block';

            // Add a button to load the next question
            const nextButton = document.createElement('button');
            nextButton.textContent = '次の問題へ';
            nextButton.className = 'option-button'; // Reuse style
            nextButton.style.gridColumn = '1 / -1'; // Span all columns
            nextButton.style.marginTop = '1rem';
            nextButton.onclick = loadQuestion;
            optionsContainer.appendChild(nextButton);


        } catch (error) {
            console.error("Failed to submit answer:", error);
            resultText.textContent = "回答の送信に失敗しました。";
        }
    }

    function setupShareButton(correct) {
        const text = correct 
            ? `たぬきクイズで正解しました！あなたも見分けられるかな？`
            : `たぬきクイズに挑戦しました！あなたも見分けられるかな？`;
        const url = window.location.href;
        const hashtags = "たぬきクイズ";
        const shareUrl = `https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}&url=${encodeURIComponent(url)}&hashtags=${encodeURIComponent(hashtags)}`;
        shareButton.href = shareUrl;
    }

    loadQuestion();
});
