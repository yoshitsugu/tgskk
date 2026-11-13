# Linux用パッケージ

対象はx86_64のArch Linux、Debian 13、Ubuntu 24.04 LTS、Ubuntu 26.04 LTSです。
各環境の標準fcitx5・IBusでビルドし、Rust toolchainはビルド用コンテナで用意します。
インストール先にRustやC++コンパイラーは不要です。
ローカルAI用workerはAVX2対応CPU向けで、モデルが利用できない場合も辞書で入力できます。

共通の推論worker・Google helper・モデル取得CLIは、フロントエンドとは別のパッケージです。
IBus版だけを入れる場合、fcitx5は依存として要求しません。
両版をインストールすることもできますが、使用する入力フレームワークはデスクトップで一方を選びます。

| 内容 | Arch | Debian / Ubuntu |
| --- | --- | --- |
| 共通プログラム | `tgskk-common-git` | `tgskk-common` |
| fcitx5版 | `tgskk-git` | `tgskk` |
| IBus版 | `tgskk-ibus-git` | `tgskk-ibus` |

## Releaseから取得

[Releases](https://github.com/yoshitsugu/tgskk/releases)で使用するOSの`tgskk-v<バージョン>-<OS>-x86_64.tar.gz`と`SHA256SUMS`を取得します。
ダウンロードしたディレクトリで、選んだアーカイブのチェックサムを確認して展開します。

```sh
# Ubuntu 24.04の例
sha256sum --ignore-missing --check SHA256SUMS
tar -xzf tgskk-v0.1.0-ubuntu-24.04-x86_64.tar.gz
```

その後、下記のインストール手順で共通パッケージと使用するフロントエンドを導入します。
Ubuntu用とDebian用のパッケージはファイル名が同じため、異なるOSのアーカイブは別のディレクトリに展開してください。

## パッケージの作成

Dockerを使い、リポジトリで次を実行します。
ホストのfcitx5や設定は変更しません。
ソースにはGitの管理対象ファイルだけを含めます。
開発中に追加したファイルを試す場合は、先に`git add`（または`git add -N`）してください。

```sh
bash scripts/build-package.sh arch
bash scripts/build-package.sh debian-13
bash scripts/build-package.sh ubuntu-24.04
bash scripts/build-package.sh ubuntu-26.04
```

成果物は`build/packages/<対象>/`に出力します。
各ビルドでテスト、IBus版単独の導入、両版の同居・再インストール・片方ずつの削除とユーザーデータの保持を確認します。
IBusは専用D-Busと一時HOME内のdaemonから、インストール済みcomponent定義を使って起動します。
GitHub Actionsの「Linux packages」も同じスクリプトを実行し、実モデルでの候補補正と採点を検証して成果物をartifactに保存します。
ローカルでも`TGSKK_TEST_MODEL`に検証済みGGUFの絶対パスを渡すと、インストール済みworkerとaddonで同じテストを実行できます。
`.deb`は対象ごとに作成したものを使ってください。

Arch向けAUR用レシピは`packaging/arch/PKGBUILD`です。
Arch上で直接作る場合は、同ディレクトリで`makepkg -si`を実行します。
このVCSレシピはGitHubのソースを取得し、固定したllama.cppのsourceとCargo依存を準備してからビルドします。
モデルの重みはパッケージに含めません。

DebianとUbuntuのパッケージ定義は`packaging/debian/`です。
標準のRustが依存crateの要件を満たさない環境があるため、コンテナにはRust 1.88を用意しています。
`dpkg-buildpackage`では、コンテナに導入したRustがdpkgの管理外であることからビルド依存の自動判定を省き、実際のビルドとテストで確認します。

## インストール

Arch Linux:

```sh
# IBus版
sudo pacman -U ./tgskk-common-git-*.pkg.tar.zst ./tgskk-ibus-git-*.pkg.tar.zst
# fcitx5版
sudo pacman -U ./tgskk-common-git-*.pkg.tar.zst ./tgskk-git-*.pkg.tar.zst
```

以前に`cmake --install`で手動導入した場合、未管理ファイルとの衝突が報告されることがあります。
`pacman -Qo <パス>`で所有者を確認し、以前導入したtgskkのプログラムを退避してから再試行します。
ユーザー辞書とモデルを退避する必要はありません。

DebianまたはUbuntu:

```sh
# IBus版
sudo apt install ./tgskk-common_*.deb ./tgskk-ibus_*.deb
# fcitx5版
sudo apt install ./tgskk-common_*.deb ./tgskk_*.deb
```

辞書とGoogle用helper、ローカルAI用workerはパッケージの依存関係と同梱物で用意します。
文脈補助を使う場合は、一般ユーザーで次を実行して検証済みモデルを取得します。

```sh
tgskk-setup-model
```

モデルはKeita Miwa氏のZenz v3.2 small（Q5_K_M、CC BY-SA 4.0）です。
固定したrevisionから取得し、SHA256を検証してから配置します。
出典とライセンスは[NOTICE](../native/zenz/NOTICE.md)を参照してください。

IBus版は、IBusを利用するセッションの入力ソースに日本語の「tgskk」を追加します。
設定と再読み込みの手順は[IBusガイド](ibus.md)を参照してください。

fcitx5版はfcitx5を再起動し、設定画面で「tgskk」を追加します。
GNOMEではfcitx5自体の起動とGTKアプリの設定も必要です。
DebianとUbuntuでは`im-config`からfcitx5を選び、ログアウトしてログインし直します。
Waylandのアプリ別設定と候補窓については[fcitx公式ガイド](https://fcitx-im.org/wiki/Using_Fcitx_5_on_Wayland/en)を参照してください。
問題の切り分けには`fcitx5-diagnose`を使えます。
GNOME Wayland上の各実アプリの表示は、パッケージ内の結合テストとは別に確認が必要です。

## 更新と削除

更新は、新しい共通パッケージと、使用中のフロントエンドのパッケージを同じコマンドで一緒に指定します。
旧fcitx5版の一体型パッケージから更新するときも、共通パッケージを追加して更新します。
両版を使用している場合は3パッケージを一緒に更新します。

削除はIBus版が`sudo pacman -R tgskk-ibus-git`または`sudo apt remove tgskk-ibus`、fcitx5版が`sudo pacman -R tgskk-git`または`sudo apt remove tgskk`です。
一方を削除しても他方と共通プログラムは残ります。
両版とも不要になったら、`tgskk-common-git`または`tgskk-common`も削除できます。
ユーザー辞書、モデル、両フロントエンドのユーザー設定は削除しません。
