# IBusで使う

IBus版はfcitx5版と同じ変換コアを使います。
最初の4候補はインライン、5候補目から候補一覧を表示します。
設定・ユーザー辞書はfcitx5版と別に管理します。

## パッケージから導入

Arch Linux、Debian 13、Ubuntu 24.04／26.04 LTS向けの作成・導入手順は[配布パッケージ](distribution.md)を参照してください。
`tgskk-common`と`tgskk-ibus`（Archでは末尾に`-git`）を一緒にインストールします。
fcitx5版を導入する必要はありません。
パッケージ導入後は、文脈補助を使う場合に`tgskk-setup-model`を一般ユーザーで実行します。

## ソースからビルドして導入

Rust 1.88以降、C++20、CMake、Ninja、pkg-config、libxkbcommon、IBus 1.5.26以降、GLibとGTK 3の開発ファイル、D-Busが必要です。
GUIテストにはXvfbを使います。Xvfbがない場合はGUIテストだけを省略し、警告します。
Debian/Ubuntuの開発パッケージは`libibus-1.0-dev libxkbcommon-dev libglib2.0-dev libgtk-3-dev dbus xvfb xauth`、Archは`ibus libxkbcommon glib2 gtk3 dbus xorg-server-xvfb xorg-xauth`です。
SKK辞書（`skkdic`または`skk-jisyo`）も用意してください。

```sh
bash scripts/check-ibus.sh
sudo cmake --install build/ibus
```

文脈補助を使う場合は推論workerも用意します。

```sh
python3 scripts/setup-zenz.py --no-model
sudo cmake --install build/zenz
tgskk-setup-model
```

モデルはfcitx5版と共通の`~/.local/share/fcitx5/tgskk/`から探し、なければ`/usr/share/tgskk/`を使います。
モデルの場所を変える場合は、下記の`ZenzModel`で指定します。

IBusを利用するデスクトップセッションへログインし直し、GNOME設定の入力ソースで日本語の「tgskk」を追加します。
既にIBusを使っている場合は`ibus restart`でも再読み込みできます。
入力フレームワークの切り替えにはログアウトが必要になる場合があります。
fcitx5とIBusを同じ入力欄で同時に使用しないでください。

## 設定

GNOMEの「設定」→「キーボード」→「入力ソース」でtgskkの設定を開けます。
`ibus-setup`でtgskkを選んで「設定」を押すこともできます。
辞書、AI、SKKサーバー、候補一覧を出すまでの候補数を変更できます。
ユーザー辞書とモデルの欄は、空欄のままなら既定パスと自動検出を使います。
不正な設定値があっても設定画面で修正でき、保存するまでは元のファイルを変更しません。
INIファイル自体の書式が壊れている場合は、読み込みエラーとして通知します。
保存しても入力中のエンジンは再起動しません。
入力を確定してから`ibus restart`を実行するか、ログインし直してください。
Fcitx5版は「Fcitx 5 設定」のtgskkの設定ボタンから変更できます。

直接編集する場合は`~/.config/tgskk/ibus.conf`に次の形式で設定します。
変更はIBusの再起動後に反映します。
`XDG_CONFIG_HOME`、`XDG_DATA_HOME`による保存先の変更にも対応します。

```ini
[General]
Dictionary=/usr/share/skk/SKK-JISYO.L
DictionaryEncoding=euc-jp
InlineCandidates=4
ModeIndicator=true
ZenzEnabled=true
SkkservMode=Google
SkkservHost=127.0.0.1
SkkservPort=1178
SkkservEncoding=euc-jp
SkkservTimeoutMs=300
```

`SkkservMode=Disabled`で外部通信を無効にし、`Custom`で独自serverを指定できます。
Googleにはローカル候補がない読みだけを送信し、周辺文脈は送りません。
`ZenzModel`でモデルの絶対パス、`UserDictionary`でユーザー辞書のパスを指定できます。
既定のユーザー辞書は`~/.local/share/ibus/tgskk/SKK-JISYO.user`です。
確定後の保存は入力処理とは別のタイマーで実行し、正常終了時にも保存します。

操作は[利用ガイド](usage.md)と共通です。
候補一覧は7件ごとに表示し、数字・クリックで確定できます。
モードはIBusのプロパティと一時的な補助表示で通知します。
表示位置や見た目はGNOME ShellなどのIBusパネルに従います。
実アプリごとの候補窓・周辺文字列の提供状況は、D-Bus結合テストとは別に確認が必要です。
