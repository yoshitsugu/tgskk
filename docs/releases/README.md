# リリース手順

1. Cargo workspace、3つのaddon（fcitx5／IBus／Windows）のCMake、fcitx5のtgskk.conf、ArchのPKGBUILD／.SRCINFO、Debian changelogのバージョンを揃えます。
   パッケージのrevision（`-2`など）はアプリケーションのバージョンとは別です。PKGBUILDと.SRCINFOのpkgver／pkgrel、および.SRCINFOに展開済みのprovides／dependsの版数も一致させます。
   `python3 scripts/check-versions.py`で、これらのバージョンがCargo workspaceと一致し、各書式から抽出できることを検査します。抽出できない書式変更や重複定義も失敗になります。
2. `docs/releases/<バージョン>.md`に利用者向けの変更点、導入方法、制約を記載します。
3. 対象commitのCI成功と、クリーンなデスクトップでの導入・入力を確認します。
4. 対象commitに`v0.1.0`のようなタグを付けてpushします。

タグのpushで4環境のパッケージをビルドし、実モデルを含む検証がすべて成功するとDraft Releaseを作成します。
OS別のアーカイブ4個と`SHA256SUMS`を添付します。
モデルの重みとユーザーデータは含めません。
現在、タグは`v<major>.<minor>.<patch>`形式に対応し、Cargo workspaceのバージョンとの一致を検査します。

Draftでタグのcommit、リリースノート、添付アーカイブとチェックサムを確認して公開します。
既存Releaseを自動で置き換えたり、タグを動かしたりはしません。
Draft作成後にジョブを再実行すると既存Releaseとの衝突で失敗するため、ビルドのやり直しが必要なら、未公開であることを確認してDraftのみを削除してから再実行します。
公開済みの内容を変更する場合は、新しいバージョンをリリースします。
