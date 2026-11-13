# READMEのデモ

`demo.gif`と`demo.mp4`は、geditでfcitx5とtgskkを使って入力した画面の録画です。
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
