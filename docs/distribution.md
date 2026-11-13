# Linux用パッケージ

対象はx86_64のArch Linux、Debian 13、Ubuntu 24.04 LTS、Ubuntu 26.04 LTSです。
各環境の標準fcitx5でビルドし、Rust toolchainはビルド用コンテナで用意します。
インストール先にRustやC++コンパイラーは不要です。
ローカルAI用workerはAVX2対応CPU向けで、モデルが利用できない場合も辞書で入力できます。

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
各ビルドでテスト、コンテナ内へのインストール、インストール済みaddonの読み込み、削除を確認します。
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
sudo pacman -U ./tgskk-git-*.pkg.tar.zst
```

以前に`cmake --install`で手動導入した場合、未管理ファイルとの衝突が報告されることがあります。
`pacman -Qo <パス>`で所有者を確認し、以前導入したtgskkのプログラムを退避してから再試行します。
ユーザー辞書とモデルを退避する必要はありません。

DebianまたはUbuntu:

```sh
sudo apt install ./tgskk_*.deb
```

辞書とGoogle用helper、ローカルAI用workerはパッケージの依存関係と同梱物で用意します。
文脈補助を使う場合は、一般ユーザーで次を実行して検証済みモデルを取得します。

```sh
tgskk-setup-model
```

モデルはKeita Miwa氏のZenz v3.2 small（Q5_K_M、CC BY-SA 4.0）です。
固定したrevisionから取得し、SHA256を検証してから配置します。
出典とライセンスは[NOTICE](../native/zenz/NOTICE.md)を参照してください。

fcitx5を再起動し、設定画面で「tgskk」を追加します。
GNOMEではfcitx5自体の起動とGTKアプリの設定も必要です。
DebianとUbuntuでは`im-config`からfcitx5を選び、ログアウトしてログインし直します。
Waylandのアプリ別設定と候補窓については[fcitx公式ガイド](https://fcitx-im.org/wiki/Using_Fcitx_5_on_Wayland/en)を参照してください。
問題の切り分けには`fcitx5-diagnose`を使えます。
GNOME Wayland上の各実アプリの表示は、パッケージ内の結合テストとは別に確認が必要です。

## 更新と削除

更新は、新しいパッケージを同じ方法でインストールしてfcitx5を再起動します。
削除は`sudo pacman -R tgskk-git`、または`sudo apt remove tgskk`です。
ユーザー辞書、モデル、fcitx5のユーザー設定は削除しません。
