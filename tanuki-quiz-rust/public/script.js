
document.addEventListener('DOMContentLoaded', () => {
    const questionEl = document.querySelector('#quiz-container h1') || document.querySelector('.container h1');
    const questionText = document.getElementById('result-message');
    const optionsContainer = document.getElementById('options');
    const shareContainer = document.getElementById('share-container');
    const shareButton = document.getElementById('share-button');

    let currentQuiz = null;

    async function loadGeneratedQuiz() {
        try {
            const res = await fetch('/api/generate_quiz');
            if (!res.ok) throw new Error('HTTP ' + res.status);
            const data = await res.json();
            currentQuiz = data;

            // display question
            if (questionEl) questionEl.textContent = 'たぬき？クイズ';
            questionText.textContent = data.question || '';

            // render choices as images
            optionsContainer.innerHTML = '';
            shareContainer.style.display = 'none';

            data.choices.forEach(choice => {
                const wrapper = document.createElement('div');
                wrapper.className = 'choice-item';
                const img = document.createElement('img');
                img.src = choice.image_url;
                img.alt = '選択肢';
                img.className = 'choice-image';
                img.onerror = () => img.style.opacity = '0.4';
                wrapper.appendChild(img);

                const btn = document.createElement('button');
                btn.textContent = 'これだ！';
                btn.className = 'option-button';
                btn.onclick = () => checkAnswer(choice);
                wrapper.appendChild(btn);

                optionsContainer.appendChild(wrapper);
            });

        } catch (err) {
            console.error('failed to load generated quiz', err);
            questionText.textContent = 'クイズの読み込みに失敗しました。';
        }
    }

    function checkAnswer(choice) {
        if (!currentQuiz) return;
        const correct = choice.category === currentQuiz.answer_category;
        if (correct) {
            questionText.textContent = '正解！おめでとう🎉';
        } else {
            questionText.textContent = `残念！正解は「${currentQuiz.answer_category}」でした。`;
        }

        // disable all buttons
        const buttons = optionsContainer.getElementsByTagName('button');
        for (let b of buttons) b.disabled = true;

        setupShareButton(correct);
        shareContainer.style.display = 'block';

        // add next question button
        const next = document.createElement('button');
        next.textContent = '次の問題へ';
        next.className = 'option-button';
        next.style.gridColumn = '1 / -1';
        next.style.marginTop = '1rem';
        next.onclick = loadGeneratedQuiz;
        optionsContainer.appendChild(next);
    }

    function setupShareButton(correct) {
        const text = correct ? 'たぬきクイズで正解しました！' : 'たぬきクイズに挑戦しました！';
        const url = window.location.href;
        const shareUrl = `https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}&url=${encodeURIComponent(url)}`;
        shareButton.href = shareUrl;
    }

    loadGeneratedQuiz();
});
