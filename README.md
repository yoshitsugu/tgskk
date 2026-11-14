# tgskk

ローカルAIを用いた変換候補順位入れ替え機能付きのSKKです。

[Zenz](https://huggingface.co/Miwa-Keita/zenz-v3.2-small-gguf)が文脈に合う辞書候補を優先します。

[![同じ読み「きしゃ」を、文脈に応じて「駅で汽車」「新聞の記者」と変換するデモ](docs/media/demo.gif)](docs/media/demo.mp4)

入力中に採点して並び変えを行い、変換候補を出すときに間に合わなければ打ち切るので、ほぼ待つことはありません。

[![50文字の文章を速く入力し、汽車・記者・景観・警官を第一候補で変換するデモ](docs/media/typing-fast.gif)](docs/media/typing-fast.mp4)

速いタイピングでも、文脈に応じて「汽車／記者」「景観／警官」が第一候補に切り替わります。

<sub>geditでの実動作を録画しています。</sub>

入力エンジンは[CSKK](https://github.com/naokiri/cskk)、デスクトップとの接続はfcitx5またはIBusを使います。
現状はLinuxのみ対応しています。
標準ルールは同梱しており、libcskkの別途インストールは不要です。
ローカル辞書に候補がない読みは、同梱の[Google IME SKK server](https://github.com/yoshitsugu/google-ime-skkserv-rs)で補います。
既定で有効です。外部へ読みを送信したくない場合は`SkkservMode=Disabled`で無効にできます。

## インストール

配布パッケージは[Releases](https://github.com/yoshitsugu/tgskk/releases)から取得できます。
使用するOSのアーカイブを選び、導入方法は[配布パッケージ](docs/distribution.md#インストール)を参照してください。

Arch Linux用PKGBUILDと、Debian 13、Ubuntu 24.04／26.04 LTS向けfcitx5版・IBus版の`.deb`の作成手順は[配布パッケージ](docs/distribution.md)を参照してください。
パッケージには推論用プログラムも含み、モデルは`tgskk-setup-model`で取得できます。

IBus版をソースからビルドする場合は[IBusガイド](docs/ibus.md)を参照してください。
以下はfcitx5版の手順です。

ソースからビルドする場合は、Rust 1.88以降、C/C++20対応コンパイラー、CMake、libxkbcommonとfcitx5 5.1.7以降の開発ファイルが必要です。
SKK辞書も用意してください。既定では`/usr/share/skk/SKK-JISYO.L`（EUC-JP）を読みます。

```sh
bash scripts/check-fcitx5.sh
sudo cmake --install build/fcitx5
```

文脈補助を使う場合は、Python 3.12以降でモデルと推論用プログラムも準備します。
セットアップ時にZenzモデル（約74 MB）とllama.cppを取得し、推論用プログラムをビルドします。入力中の推論はローカルで完結します。

```sh
python3 scripts/setup-zenz.py
sudo cmake --install build/zenz
```

fcitx5を再起動し、設定画面で「tgskk」を入力メソッドに追加してください。
モデルを使わない場合も辞書順で入力できます。文脈補助は`ZenzEnabled`で無効にできます。

## 操作

Shiftを押しながら読みの入力を始め、Spaceで変換します。
最初の4候補は入力位置で順に切り替わり、5候補目から一覧を表示します。

| 場面 | キー | 動作 |
| --- | --- | --- |
| 一覧を出す前 | `Space` / `x` | 次 / 前の候補 |
| 一覧表示中 | `Space` / `x`、`Page_Down` / `Page_Up` | 次 / 前のページ |
| 候補選択中 | 上下キー | 候補を移動 |
| 候補選択中 | `Return` | 確定 |
| 一覧表示中 | `1`〜`7`、クリック | 候補を選んで確定 |
| 入力・変換中 | `C-g` | キャンセル |

`InlineCandidates`で一覧を出す前の候補数を変更できます。既定は4、3ならfcitx5-cskkと同じタイミング、0なら最初から一覧を表示します。
AIが実際に順位を変えた場合は、一覧の補助行に *AI-ranked* と表示します。表示後の候補順は変わりません。

辞書・文脈補助・SKK serverの設定、復旧手順、CLIの使い方は[利用ガイド](docs/usage.md)を参照してください。

## 開発

```sh
bash scripts/check.sh
bash scripts/check-fcitx5.sh
```

Rustの静的検査は`bash scripts/lint.sh`、依存監査は`cargo install cargo-deny --version 0.18.9 --locked`の後に`bash scripts/check-dependencies.sh`で実行できます。
CIの静的検査にはRust 1.99.1を使用し、ビルドの最低対応版は1.88のままです。

C/C++の整形と静的解析にはclang-format / clang-tidy 23.1.4を使います。
`python3 scripts/lint-cpp.py format`で整形を検査し、`--fix`を付けると整形できます。
静的解析はCMakeの構成時に`-DCMAKE_EXPORT_COMPILE_COMMANDS=ON`を指定してから、
`python3 scripts/lint-cpp.py tidy build/ibus build/fcitx5 build/zenz`で実行します。
WindowsではVisual Studioの開発者シェルでNinjaを使って構成し、Windows用のビルドディレクトリを渡してください。
`C++ quality`ワークフローがLinuxとWindowsのコード・テストを解析します。外部依存コードは対象外です。

実モデルを用意した環境では`bash scripts/check-zenz.sh`で結合テストと採点・入力処理の測定を実行できます。

- [速度回帰検査](benchmarks/README.md)
- [設計](docs/architecture.md)
- [CSKKの取り込み情報](third_party/cskk/UPSTREAM.md)

## ライセンス

[GPL-3.0-or-later](LICENSE)。CSKKの著作権情報は[LICENSE.txt](third_party/cskk/LICENSE.txt)と[README](third_party/cskk/README.md)、モデルと推論runtimeの出典・ライセンスは[NOTICE](native/zenz/NOTICE.md)に記載しています。
同梱するGoogle IME SKK serverの出典とライセンスは[取り込み情報](third_party/gskkserv/UPSTREAM.md)を参照してください。
