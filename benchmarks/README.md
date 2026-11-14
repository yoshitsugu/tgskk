# 速度回帰検査

CIでは、同じLinux runnerで基準コミットと変更後のreleaseビルドを交互に7組測ります。
PRでは対象ブランチの先端、pushではpush前のコミット、週次実行では直前のコミットを基準にします。
両方に同じ測定コードと辞書を使い、RustとC++のツールチェーンも揃えます。

C ABI経由で、かな入力、通常の変換、512候補ある読みをAI有効／無効で測ります。
打鍵、Space、確定の時間を分けて記録します。
辞書には合成した1万件の見出しを使い、ユーザー辞書や個人の設定は使いません。
SKK serverは有効にしたまま測り、ローカル辞書へのヒットで外部問い合わせが発生した場合も失敗させます。
実モデルの採点も、文脈の先読み後に4候補と8候補で比較します。

各組のp50が25%、p99が50%を超えて遅くなり、かつ差が1µsを超える結果が7組中5組以上あればCIを失敗させます。
測定値と比較結果はActionsのサマリーと`performance-comparison`成果物に残します。
共有runnerの揺れを抑えるための判定なので、小幅な劣化の検出や実機での体感評価を置き換えるものではありません。
C ABIの外にあるフロントエンドの処理やプロセス間通信は、この相対比較の対象外です。

ローカルでは基準となるチェックアウトを用意し、次の順に実行します。
同じRust toolchainを両ビルドに使ってください。モデルとllama.cppの取得にはネットワークが必要です。

```sh
bash scripts/build-performance.sh /path/to/reference-checkout
python3 scripts/check-performance.py \
  --baseline-input build/performance/baseline/input \
  --candidate-input build/performance/candidate/input \
  --baseline-worker /path/to/reference-checkout/build/zenz/tgskk-zenz-worker \
  --candidate-worker build/zenz/tgskk-zenz-worker \
  --model build/models/zenz-v3.2-small-Q5_K_M.gguf \
  --baseline-revision BASE_SHA --candidate-revision HEAD_SHA \
  --output build/performance/comparison.json
```
