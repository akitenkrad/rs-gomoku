[English](development.md) | **日本語**

# 開発者向け情報

## ビルド

```sh
cargo build --release
cargo run --release                      # play
cargo run --release -- replay FILE       # replay a record
```

rs-gomoku は Rust edition 2024 で，描画に [ratatui](https://ratatui.rs)，コマンドラインに [clap](https://docs.rs/clap) を使っています．構成は [rs-nim](https://github.com/akitenkrad/rs-nim) と [rs-tic-tac-toe](https://github.com/akitenkrad/rs-tic-tac-toe) にならっています．

## ソース構成

| ファイル | 役割 |
|---|---|
| `src/main.rs` | コマンドラインの解析，イベントループ（33 ms ごとの tick），`replay --print` |
| `src/lib.rs` | 以下のモジュールをライブラリとして公開する（結合テストのため） |
| `src/game.rs` | 7〜19 路の盤，ルール（自由・標準），座標，勝ち判定．UI に依存しない |
| `src/search.rs` | パターン評価，候補手の生成，反復深化付きの αβ 探索，VCF 探索 |
| `src/ai.rs` | 難易度，別スレッドでの CPU の着手選択，小さな xorshift 乱数 |
| `src/record.rs` | `.gmkrec` 棋譜の読み書きと，任意の手数の局面の再構成 |
| `src/i18n.rs` | 画面に出るすべての文言の英語と日本語 |
| `src/app.rs` | 画面遷移，入力，通常モードとリプレイモード，バックグラウンド分析，アニメーション，セッション戦績，紙吹雪と雨 |
| `src/ui.rs` | 描画と，盤のサイズに応じた必要端末サイズの計算 |

CPU・ヒント・評価パネルの探索は別スレッドで動きます．要求には対局 ID を付けているので，新しい対局を始めたりタイトルに戻ったりした後に届いた古い結果は捨てられます．

## テスト

```sh
cargo test                              # unit and integration tests
cargo test -- --ignored --nocapture     # text dump of the screens and a full game against the CPU
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

結合テストは `tests/` にあります．

| ファイル | 確かめること |
|---|---|
| `game.rs` | 4 方向の五，盤の端と隅での五，両ルールでの長連，引き分け，不正な手，ランダム対局での勝ち判定と素朴な実装の照合 |
| `search.rs` | 形の分類，五を作る，相手の五を止める，活三を受ける，VCF を見つける，証明していない勝ち負けを返さない |
| `ai.rs` | どの難易度も合法手だけを指す，つよいが制限時間内に答える |
| `record.rs`，`record_i18n.rs` | パース，行番号付きの各エラー，往復，全サンプル棋譜，言語ごとの既定名 |
| `app_ui.rs`，`app_i18n.rs`，`i18n.rs` | リプレイの移動，保存，バックグラウンド思考，9・15・19 路の最小サイズでの全画面の描画（両言語） |
| `cli.rs` | `replay --print` の出力とエラー |
| `regression/` | テスト中に見つかった境界条件・状態遷移・別スレッドのタイミング |
