# READMEのデモ

`demo.gif`と`demo.mp4`は、geditでfcitx5とtgskkを使って入力した画面の録画です。
`typing-fast.gif`と`typing-fast.mp4`は、IBus版で50文字の文章を入力した録画です。
SKK-JISYO.L（EUC-JP）とローカルのZenzモデルを使っています。
キー入力はXTestで自動化し、変換中の表示も含めて等速で収録しています。

## 撮り直す

Linux、Python 3、Xvfb、libX11、libXtst、D-Bus、gedit、fcitx5のGTK3モジュール、ffmpeg、Noto Sans CJK JPが必要です。
`/usr/share/skk/SKK-JISYO.L`も用意してください。

モデルとworkerを[セットアップ](../../README.md#インストール)してから、リポジトリのルートで実行します。

```sh
bash scripts/check-zenz.sh
python3 scripts/record-demo.py
```

録画用のX11画面、D-Busセッション、設定・学習辞書を一時ディレクトリに作ります。
geditはOblivion配色、Noto Sans CJK JP 32ptで表示します。
出力は1120×480のGIFとMP4で、保存先は`--output-dir`で変更できます。

```sh
python3 scripts/record-demo.py --output-dir /tmp/tgskk-demo
```

## 速い入力のデモ

IBus、GTK3のIBusモジュール、PythonのGObject Introspection（IBus typelibを含む）も必要です。
上記のモデルとworkerに加え、録画用のIBusエンジンをビルドします。

```sh
bash scripts/check-ibus.sh
cmake -S addons/ibus -B build/ibus-model-test -DCMAKE_BUILD_TYPE=Release \
  -DTGSKK_ZENZ_WORKER="$PWD/build/zenz/tgskk-zenz-worker"
cmake --build build/ibus-model-test
python3 scripts/record-demo.py --scenario typing --interkey-ms 40
```

文章は「駅で汽車と紅葉の景観を撮影する老人。その老人を取材する記者は、交通違反を調査する警官にも話をきいた。」です。
Space・確定を含む149キーを同じ間隔で送り、候補を選び直さず入力します。
空のユーザー辞書と新しい推論workerで始め、モデルの起動後に入力を開始します。
geditが保存した全文が期待どおりの場合だけ動画を出力します。

補正が間に合う間隔はCPUや負荷によって変わります。
`--trial`を付けると動画を作らずに確認でき、`--interkey-ms`で間隔を調整できます。
