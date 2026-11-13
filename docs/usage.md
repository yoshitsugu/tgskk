# 利用ガイド

## fcitx5の設定

fcitx5の設定画面から「tgskk」の設定を開きます。
設定ファイルを直接編集する場合の通常の場所は`~/.config/fcitx5/conf/tgskk.conf`です。

| 項目 | 既定値 | 用途 |
| --- | --- | --- |
| `Dictionary` | `/usr/share/skk/SKK-JISYO.L` | SKK辞書のパス |
| `Encoding` | `euc-jp` | `euc-jp`または`utf-8`（小文字で指定） |
| `InlineCandidates` | `4` | 一覧を出す前に順に表示する候補数。`0`なら最初から一覧 |
| `ModeIndicator` | `True` | モード変更時に「あ」「ア」「A」などを一時表示 |
| `SkkservMode` | `Google` | 候補がないときの取得先。`Google`、`Custom`、`Disabled` |
| `SkkservHost` | `127.0.0.1` | `Custom`で使うserverのホスト |
| `SkkservPort` | `1178` | `Custom`で使うserverのポート |
| `SkkservEncoding` | `euc-jp` | `Custom`の通信文字コード。`euc-jp`または`utf-8` |
| `SkkservTimeoutMs` | `300` | 候補取得を待つ時間（0〜1000ms） |
| `ZenzEnabled` | `True` | ローカルAIによる候補の並べ替え |
| `ZenzModel` | `/usr/share/tgskk/zenz-v3.2-small-Q5_K_M.gguf` | v3形式に対応するモデルのパス |

配布パッケージでは、`tgskk-setup-model`でモデルを取得できます（sudo不要）。
既定の保存先は`~/.local/share/fcitx5/tgskk/`です。
`ZenzModel`が既定値の場合は、このディレクトリのモデルを優先します。
取得後はfcitx5を再起動してください。
`--directory`で別の場所を指定した場合は、`ZenzModel`にもそのファイルのパスを指定します。

辞書ファイルの内容を同じパスで更新した場合は、fcitx5を再起動して読み直してください。
モード表示や`InlineCandidates`の変更では、入力途中の文字を保持します。
モード表示の位置・期間と補助行の書体・文字サイズは、fcitx5のUIに従います。
fcitx5 5.1.7〜5.1.8では注釈を候補の後ろに表示し、5.1.10以前のモード通知はfcitx5共通の表示設定にも従います。
GNOME Waylandの実アプリでの表示は未検証です。

### SKK server

ローカル辞書に候補がないときだけ、オンラインで候補を取得します。
既定の`Google`は同梱のGoogle IME SKK serverを使い、変換対象の読みをGoogleへ送信します。
送り仮名がある場合は送り仮名を含む読みを送ります。
左文脈や確定済みの文章は送信しません。
`Google`で取得するのは、ひらがなと長音符などからなる読みの候補です。
外部取得を使わない場合は`SkkservMode=Disabled`にします。

辞書に候補がある通常の変換と、読みを入力している間には通信しません。
候補取得が既定の300msに間に合わなければ、通常の辞書登録へ進みます。
表示済みの候補は後から差し替えません。
遅れて届いた結果は次回の変換で利用でき、`SkkservTimeoutMs=0`では待たずに取得を依頼します。
取得結果はメモリに最大128件、候補がある場合は5分、候補がない場合は5秒保持します。
通信失敗は候補なしとしてcacheせず、次回に再試行します。
取得先などの設定変更と再起動でcacheを破棄します。

独自のSKK serverを使う場合は、serverを別途起動し、次のように設定します。
通信文字コードは辞書の`Encoding`とは独立しています。

```ini
SkkservMode=Custom
SkkservHost=127.0.0.1
SkkservPort=1178
SkkservEncoding=utf-8
SkkservTimeoutMs=300
```

Google用helperは最初の取得時に起動し、その後は再利用します。
ポートの確保や別途のdaemon起動は不要で、helperとの通信にはUTF-8を使います。
文脈補助を無効にしてもSKK serverは使えます。
password、sensitive、disable属性、選択範囲がある入力欄と、フォーカスのない入力欄では、cacheを含めて外部候補を使いません。
外部候補も確定するとユーザー辞書へ学習します。
数値変換用の`#`テンプレートなど、外部候補として扱えない表記は除外します。
取得に失敗してもローカル辞書での入力は継続でき、診断はfcitx5のログへ出ます。

### 候補一覧

既定では最初の4候補をインラインで巡回し、5候補目から残りの候補を一覧に出します。
この既定値は[AquaSKK](https://github.com/codefirst/aquaskk/blob/master/platform/mac/plist/UserDefaults.plist)に合わせています。
`InlineCandidates=3`にすると、[fcitx5-cskkの既定](https://github.com/fcitx/fcitx5-cskk/blob/master/src/cskkconfig.h)と同じタイミングになります。

インライン候補がある場合、一覧の先頭ページで`x`または`Page_Up`を押すと、最後のインライン候補へ戻ります。
候補が4件以下なら既定では一覧を出さず、最後の候補の次で辞書登録へ進みます。
最後のページで`Space`を押すと辞書登録へ進みます。
インライン表示中の`Page_Down`は残りの候補一覧を開き、候補が少なく一覧がない場合は最後の候補へ移動します。
`Page_Up`はインライン表示中には何もせず、どちらのページキーもアプリへは渡しません。
上下キーは候補の先頭と末尾で止まります。
ホイールでページを変えた場合、入力位置の候補表示は次のキーで同期します。
インライン表示中は、`AI-ranked`を含めて補助行を表示しません。
インライン候補だけで確定した場合、AIが働いていても表示は出ません。
フォーカスを失うと未確定の入力は取り消します。

### ユーザー辞書

学習用のUTF-8辞書は、fcitx5のユーザーデータディレクトリ内の`tgskk/SKK-JISYO.user`です。
通常は`~/.local/share/fcitx5/tgskk/SKK-JISYO.user`に保存されます。
確定ごとには書き込まず、fcitx5の保存処理で書き出します。
保存時は一時ファイルを完成させてから置き換え、直前の内容を`.BAK`へ残します。

## 文脈補助

Zenzは上位8候補までを採点し、差が大きい場合だけ第一候補を昇格させます。
例えば「駅で」の後の「きしゃ」では「汽車」を、「新聞の」の後では「記者」を優先する効果を確認できます。
先読みが間に合わない場合や判断が曖昧な場合は辞書順を維持します。
送り仮名付き変換や英数字を含む読み・候補は、現在は辞書順で扱います。

ユーザー辞書にしか存在しない候補の位置は変更しません。
通常辞書にもある学習語は文脈補助の対象です。
AIが変更した第一候補をそのまま確定しても通常の候補順には学習せず、明示的に選び直した場合は学習します。

### 参照する文脈

カーソル左側の最大40文字を使います。文ごとには区切らず、その範囲に入る前の文も参照します。
周辺文字列を取得できないアプリでは、同じ入力欄でtgskkが確定した文字を使います。
フォーカス移動や未処理キーで文脈を破棄し、password・sensitive属性や選択範囲のある入力欄ではAIを停止します。
文脈と採点結果はメモリ内だけに保持します。

### モデルが使えない場合

辞書順で入力を続けられ、候補一覧には「Zenz unavailable」と表示します。
候補一覧を開かない間は通知も出ないため、モデルが使えないと疑う場合はfcitx5のログや採点用CLIで確認してください。
ログに出す障害理由には入力内容を含めません。
一時的な通信障害では、背景で1秒・2秒・4秒・15秒と間隔を延ばして再起動します。
その後も30秒間隔で回復を試みます。
連続して4回失敗すると候補一覧に利用不能を表示しますが、自動回復は続きます。
採点に成功した後の障害では、再起動の待ち時間を1秒に戻します。

モデルや推論用プログラムが見つからない場合、または再試行しても回復しない場合は、配置を確認してから`ZenzEnabled`を一度無効にして有効に戻すか、fcitx5を再起動してください。
既定の推論用プログラムは`/usr/libexec/tgskk-zenz-worker`です。

### 採点だけを確認する

```sh
cargo run --release -p tgskk-zenz --bin tgskk-zenz-eval -- \
  build/zenz/tgskk-zenz-worker build/models/zenz-v3.2-small-Q5_K_M.gguf \
  '駅で' 'キシャ' '記者' '汽車' '帰社' '貴社'
```

出力は候補ごとの対数尤度で、大きい値ほどモデルが優先します。

## CLIで試す

Rust toolchain、Cコンパイラー、libxkbcommonの開発ファイルが必要です。

```sh
cargo build --release -p tgskk-cli
./target/release/tgskk keys examples/SKK-JISYO.demo.utf8 utf-8 \
  'K a n j i space space Return'
```

この例では「漢字」から「感じ」へ候補を進めて確定します。
キー列は空白区切りのkeysym名で、大文字はShift入力を表します。
CLIは入力状態を出力し、ほかのアプリへ文字を送るものではありません。
fcitx5の一覧表示の設定はCLIには適用されません。

実際の辞書やモード切り替えも試せます。

```sh
./target/release/tgskk keys /usr/share/skk/SKK-JISYO.L euc-jp 'K a K u Return'
./target/release/tgskk keys examples/SKK-JISYO.demo.utf8 utf-8 'q l C-j'
```

読みからの簡易検索は、UTF-8辞書の基本的な`見出し /候補;注釈/候補/`形式に対応します。

```sh
./target/release/tgskk examples/SKK-JISYO.demo.utf8 かんじ
```

EUC-JP、送り仮名ブロック、数値変換には`keys`コマンドを使ってください。

## 性能の確認

```sh
bash scripts/check-zenz.sh
cargo bench -p tgskk-core --bench lookup
```

`check-zenz.sh`はモデルと推論用プログラムを準備した環境で実行します。
同梱の例文を採点し、打鍵間隔60msと0msの先読み命中率・Rustのキー処理時間を比較します。
`/usr/share/skk/SKK-JISYO.L`がある場合は、実辞書でも30msと60msの間隔で測定します。
例文は小さな動作確認用セットで、一般的な変換精度を保証するものではありません。
`lookup`は簡易検索用の`Dictionary`を測るベンチマークで、CSKKの入力経路は含みません。
いずれも実アプリの描画は測定しません。
