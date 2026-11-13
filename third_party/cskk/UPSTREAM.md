# CSKKの出典とtgskkでの変更

上流は[naokiri/cskk](https://github.com/naokiri/cskk)。
取り込み元のrevisionは`e99d1ba7c7b0a1b243dd096742e32025d9a282ef`（4.0.0）。
ライセンスと著作権情報は、元の[LICENSE.txt](LICENSE.txt)、README、Cargoの著者情報を保持している。
入力エンジン、標準ルール、辞書処理、LRU map、既存テストとfixtureを取り込んだ。
上流のCI、リリーススクリプト、パッケージ作成用ファイルは含めていない。

## tgskk向けの拡張

- `conversion`モジュールに、表示前に取得済み候補と順位を渡す同期providerを追加した。
- ローカル候補がない実変換だけで呼ぶ`MissingConversionProvider`を追加した。previewやcompletionでは呼ばず、追加した候補を通常のproviderへ渡す。`MissingConversionResponse::Pending`を返すと登録へ移らず、読みと送り仮名を保って次の明示的な変換操作で再試行する。従来の`ConversionResponse`を返すclosureは完了済み応答として使える。
- SKK serverとの連携用に、既存の送り仮名見出し生成へ委譲する`ConversionRequest::dictionary_key()`と、外部候補の共通検証`ConversionCandidate::validate_external()`を公開した。
- 順位のindexは`request.candidates`の順序を基準とする。ローカル候補は注釈や数値変換の由来を保つため重複を残し、表記がまだない追加候補だけを順番に末尾へ足す。この一覧の完全な順位置換だけを受け入れる。
- 不正な追加候補や順位では応答全体を捨て、ローカル候補を保つ。エラーは呼び出し元へ返せる。
- 外部候補の`/`、`;`、括弧は辞書の既存エスケープ処理で保存する。保存後の再解釈を防ぐため、表記と注釈の空文字、制御文字、`[]`、バックスラッシュ、`(concat`を含む文字列は拒否する。表記の`#`も数値変換と混同するため拒否する。
- 内部候補一覧へ順位を反映するため、Space巡回、逆方向、登録への遷移、確定で同じ順序を使う。
- `Always`と`ExplicitSelectionOnly`で確定時の学習を分ける。`learning: None`では統合後の第一候補が元の辞書の第一候補と同じなら前者、変わった場合は後者になる。表記が同じでも、見出しや数値変換前の候補が異なれば区別する。後者でも、巡回やindex指定による明示選択は通常の学習対象になる。
- 候補の内容をRustから読むgetterと、標準ルールを埋め込んだ初期化APIを追加した。
- `preview_conversion`は▽中の読みと候補を、選択や学習をせずに返す。保留中の`n`もSpaceと同じ規則で先読みする。
- `preview_conversion_limited`では返す候補数を制限できる。実変換のproviderへは引き続き全候補を渡す。
- `ConversionRequest::protected_indices`は、読み取り専用の辞書に存在しない候補の位置を返す。通常辞書にもある学習語と、独自登録語をproviderが区別できる。
- 辞書候補の重複除去後の順序復元と保護対象の判定には集合を使い、候補数に対して二乗になる走査を避ける。
- fcitx5のrawキーイベントをRustから生成できるようにし、辞書の保存失敗を呼び出し元へ返す`CskkDictionary::save`を追加した。
- 直接入力中の取消では登録の入れ子を抜ける場合を除いて、終了済みの変換履歴へ戻らない。かな確定や候補削除の後に古い▽や空の▼が復活することを防ぐ。
- ユーザー辞書は同じディレクトリの一時ファイルへ書き出して同期し、元の内容のバックアップを保存した後、ファイルを置き換える。元のファイルが消えていてもメモリ上の学習から再保存できる。符号化や保存に失敗した場合は元のファイルと未保存の学習を保持する。

- `CskkDictionary`に非同期保存用のstreamを追加した。初期状態を一度だけコピーし、以降は変更した見出しごとの最終状態をまとめてwriterへ渡す。LRUの変更順と辞書のrevisionを保持し、古い保存完了通知で新しい学習を保存済みにしない。stream中の同期保存とreloadは拒否し、保存失敗は未保存の変更を保持して再試行できる。

通常のproviderは入力スレッドで呼ばれる。
ネットワーク、ディスク、推論をこの中で実行せず、workerやcacheで準備済みの結果だけを返す契約である。
tgskkのZenz providerもこの契約に従い、バックグラウンドの採点結果だけを読む。
不足候補用のproviderでは、ローカル候補がない場合に限り、バックグラウンド取得を指定時間だけ待てる。
providerは候補一覧の初回構築時に呼ばれ、すでに表示している変換中の一覧を遅着結果で変更するAPIは用意していない。
読みが空の場合はproviderを呼ばず、候補選択へも移行しない。
統合時に重複として捨てる追加候補も検証対象になるため、providerはそれらの注釈も辞書で往復できる形に整える。

## ビルドと検証の調整

Rustライブラリとしての利用では不要な`cargo-c`のbuild dependencyを外し、xkbcommonのWayland向けfeatureを無効にした。
Unixではキーの解釈に必要なlibxkbcommonへの依存が残る。
テスト用loggerのdefault featureも外した。
元のCargoパッケージ用metadataは保持しているが、リリース用の`cargo cinstall`やDebianパッケージ生成は検証していない。

tgskkのworkspaceからは独立させ、上流テストを別のworkspaceとして実行する。
`bash scripts/check.sh`で両方のテストを実行できる。

## Windows portability

Windows builds use a Rust keysym adapter instead of linking libxkbcommon.
Its key constants and name table are generated from the libxkbcommon header;
the original notices are retained in `cskk/src/portable_keysyms_data.rs`.
The generator pins the libxkbcommon 1.13.2 header by SHA-256. Parity tests compare
the common names with the installed library; newer names and control/numeric
key mappings are checked independently when that library predates them.
Unix builds continue to use libxkbcommon. XDG-based rule discovery is Unix-only;
Windows callers use bundled rules or an explicit rule directory.
The read-only eligibility query includes uppercase kana starts and ASCII
fallback input. It is conservative: the processing result still determines
whether a key is consumed, since commands may pass keys through after effects.
