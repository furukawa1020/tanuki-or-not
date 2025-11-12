
document.addEventListener('DOMContentLoaded', () => {
    const quizImage = document.getElementById('quiz-image');
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
            // hide the top quiz-image (we show choices instead)
            if (quizImage) { quizImage.style.display = 'none'; }
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
                // If external image fails (CDN/Heroku/Unsplash error), fall back to local server-generated image.
                img.onerror = () => {
                    try {
                        img.src = `/images/${choice.category}.png`;
                    } catch (e) {
                        img.style.opacity = '0.4';
                    }
                };
                wrapper.appendChild(img);

                const btn = document.createElement('button');
                btn.textContent = 'これだ！';
                btn.className = 'option-button';
                btn.onclick = () => checkAnswer(choice);
                wrapper.appendChild(btn);

                optionsContainer.appendChild(wrapper);
            });

            // make sure the options container is visible
            optionsContainer.style.display = '';

        } catch (err) {
            console.warn('failed to load /api/generate_quiz - falling back to client-side source images', err);
            // Fallback: generate choices locally using Unsplash Source URLs so the UI works even if the Rust server isn't running
            const fallbackCategories = [
                { key: 'forest', keywords: 'forest,trees' },
                { key: 'water', keywords: 'ocean,sea,water' },
                { key: 'urban', keywords: 'city,street,building' },
                { key: 'animal', keywords: 'animal,wildlife' },
            ];
            const rng = () => Math.floor(Math.random() * 1e9);
            const choices = fallbackCategories.map((c, idx) => ({
                id: idx + 1,
                image_url: `https://source.unsplash.com/800x600/?${c.key},${c.key}&sig=${rng()}`,
                category: c.key,
            }));
            // pick random answer
            const answer_category = choices[Math.floor(Math.random() * choices.length)].category;
            currentQuiz = { question: '次の画像のうち、どれが該当しますか？', choices, answer_category };

            // render fallback
            if (questionEl) questionEl.textContent = 'たぬき？クイズ';
            questionText.textContent = currentQuiz.question || '';
            optionsContainer.innerHTML = '';
            choices.forEach(choice => {
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
        }
    }

    async function checkAnswer(choice) {
        if (!currentQuiz) return;

        // disable all buttons immediately to avoid double-clicks
        const buttons = optionsContainer.getElementsByTagName('button');
        for (let b of buttons) b.disabled = true;

        // try server-side authoritative validation first
        try {
            const res = await fetch('/api/submit_generated', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ selected_category: choice.category, answer_category: currentQuiz.answer_category })
            });

            if (res.ok) {
                const data = await res.json();
                const correct = data.correct;
                if (correct) {
                    questionText.textContent = '正解！おめでとう🎉';
                } else {
                    questionText.textContent = `残念！正解は「${data.correct_answer}」でした。`;
                }
                setupShareButton(correct);
                shareContainer.style.display = 'block';
            } else {
                // server returned non-OK: fallback to client-side
                console.warn('/api/submit_generated returned', res.status);
                fallbackLocalCheck(choice);
            }
        } catch (err) {
            // network error or server unreachable: fallback
            console.warn('failed to POST /api/submit_generated - falling back', err);
            fallbackLocalCheck(choice);
        }

        // add next question button
        const next = document.createElement('button');
        next.textContent = '次の問題へ';
        next.className = 'option-button';
        next.style.gridColumn = '1 / -1';
        next.style.marginTop = '1rem';
        next.onclick = loadGeneratedQuiz;
        optionsContainer.appendChild(next);
    }

    function fallbackLocalCheck(choice) {
        const correct = choice.category === currentQuiz.answer_category;
        if (correct) {
            questionText.textContent = '正解！おめでとう🎉';
        } else {
            questionText.textContent = `残念！正解は「${currentQuiz.answer_category}」でした。`;
        }
        setupShareButton(correct);
        shareContainer.style.display = 'block';
    }

    function setupShareButton(correct) {
        const text = correct ? 'たぬきクイズで正解しました！' : 'たぬきクイズに挑戦しました！';
        const url = window.location.href;
        const shareUrl = `https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}&url=${encodeURIComponent(url)}`;
        shareButton.href = shareUrl;
    }

    loadGeneratedQuiz();
});
