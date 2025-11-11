from flask import Flask, render_template, request, jsonify
import random

app = Flask(__name__)

# 仮の画像データ。実際には画像ファイルへのパスなどを格納します。
animals = [
    {'name': 'タヌキ', 'image': 'tanuki.jpg'},
    {'name': 'アナグマ', 'image': 'anaguma.jpg'},
    {'name': 'ハクビシン', 'image': 'hakubishin.jpg'},
]

@app.route('/')
def index():
    """クイズページの初期表示"""
    # 動物をランダムに1つ選ぶ
    correct_animal = random.choice(animals)
    
    # 選択肢を作成（正解を含む3択）
    options = random.sample([animal['name'] for animal in animals], 3)
    if correct_animal['name'] not in options:
        # 正解が選択肢に含まれていない場合、ランダムな1つを置き換える
        replace_index = random.randint(0, 2)
        options[replace_index] = correct_animal['name']

    return render_template('index.html', animal=correct_animal, options=options)

if __name__ == '__main__':
    app.run(debug=True)
