[English](replay.md) | **日本語**

# 棋譜とリプレイ

rs-gomoku はテキスト形式の棋譜ファイル（`.gmkrec`）から対局を再生できます．棋譜は手で書いても，別のプログラムで生成しても，自分の対局から保存してもかまいません．リプレイは録画にも使えるように作ってあり，一定の間隔で自動再生し，最後まで再生したら自動で終了できます．

## 棋譜の再生

```sh
rs-gomoku replay examples/records/black-wins-15.gmkrec
```

盤のサイズとルールは棋譜に書かれたものを使います．

| オプション | 説明 |
|---|---|
| `<FILE>` | 棋譜ファイル．`-` で標準入力から読む |
| `--interval <INTERVAL>` | 自動再生の 1 手あたりの間隔（ミリ秒．既定 `1500`．石が現れるアニメーションより短い値は，その長さに切り上げ） |
| `--paused` | 一時停止の状態で開始 |
| `--exit-on-end` | 終局した棋譜の最後まで再生したら，3 秒後に終了 |
| `--print` | TUI を起動せず，着手と最終盤面をテキストで出力 |
| `--lang <LANG>` | 表示言語．`en`（既定）か `ja`．`--print` の出力にも効く |

ファイルが読めない場合や棋譜に誤りがある場合は，TUI を起動する前に問題点（行番号付き）を標準エラーに出し，終了コード 1 で終わります．

### リプレイ中の操作

| キー | 動作 |
|---|---|
| `Space` | 再生 / 一時停止 |
| `→` / `l` | 1 手進む（アニメーション付き） |
| `←` / `h` | 1 手戻る |
| `Home` / `g` | 先頭へ |
| `End` / `G` | 末尾へ |
| `+` / `-` | 速く / 遅く（1 段 250 ms） |
| `e` | 評価パネルの表示切替 |
| `q` / `Esc` / `Ctrl-C` | 終了 |

ステータス欄には再生中かどうか，手数（`k/N`），現在の間隔を表示します．最後の手で終局していれば，結果パネルに勝者の名前か引き分けを表示します．途中で終わる棋譜では「棋譜の終わり」と表示します．

### テキスト出力

`--print` は棋譜を検査し，すべての着手と，最終盤面（`X` = 黒，`O` = 白），結果を出力します．

```sh
rs-gomoku --lang ja replay examples/records/small-9.gmkrec --print
```

```text
手数 1: Bob 黒 e5
手数 2: Alice 白 e4
...
手数 5: Bob 黒 d4

. . . . . . . . .
. . . . . . . . .
. . . . . . . . .
. . . X O . . . .
. . . X X O . . .
. . . . . . . . .
. . . . . . . . .
. . . . . . . . .
. . . . . . . . .
結果: 未終局
```

## 自分の対局を保存する

対局中または結果画面で `s` を押すと，現在の対局を `gomoku-<unix 秒>.gmkrec` として保存します．保存先はカレントディレクトリです．`--record-dir <DIR>` を付けて起動すると変えられます．

```sh
rs-gomoku --record-dir ~/gomoku-records
```

保存した棋譜には既定の名前を書きません．そのため英語では You / CPU，日本語では あなた / CPU と表示されます．名前を付けるには `human` / `cpu` の行を追加します．

## `.gmkrec` 形式

棋譜は UTF-8 のテキストで，`key = value` 形式のヘッダの後に，1 行 1 手で着手を並べます．

```text
# rs-gomoku record v1
size  = 15         # 7..19                 (default: 15)
rule  = freestyle  # freestyle | standard  (default: freestyle)
first = human      # human | cpu           (default: human; the first player plays Black)
human = Alice      # optional display name of the "human" side (default: You / あなた)
cpu   = Bob        # optional display name of the "cpu" side   (default: CPU)

h8                 # move: column letter + row number, row 1 at the top
I9                 # case-insensitive
```

規則:

- `#` 以降はコメント，空行は無視します．
- ヘッダのキーは最初の着手より前にだけ書けます．未知のキー・重複したキー，7〜19 以外のサイズはエラーです．
- 着手は `first` から交互で，`first` が黒を持ちます．座標は [盤のサイズと座標](gameplay.ja.md#盤のサイズと座標) を参照してください．
- `human` と `cpu` は 2 つの陣営を表すだけです．リプレイでは指定した名前で表示するので，任意の 2 人の対局を記録できます．
- 盤外の点・埋まっている点への着手と，終局後の着手はエラーです．終局前で終わる棋譜は許されます．

[`examples/records/`](../examples/records/) のサンプル棋譜:

| ファイル | 内容 |
|---|---|
| `black-wins-15.gmkrec` | 15 × 15，自由ルール．黒が四三で勝つ |
| `small-9.gmkrec` | 9 × 9 の盤での短い対局 |
| `overline-standard.gmkrec` | 9 × 9，標準ルール．長連ができても対局が続く |
| `unfinished.gmkrec` | 途中で終わる棋譜 |

## 録画する

リプレイを使えば同じ対局を何度でも再現できるので，録画のやり直しも簡単です．録画する端末は盤に足りる大きさにしてください（[端末の要件](gameplay.ja.md#端末の要件) を参照）．

[asciinema](https://asciinema.org) の場合:

```sh
asciinema rec -c "rs-gomoku replay game.gmkrec --exit-on-end" gomoku.cast
```

[VHS](https://github.com/charmbracelet/vhs) の場合は，次のようなテープファイルを使います．

```text
Output gomoku.gif
Set Width 1400
Set Height 800
Type "rs-gomoku replay game.gmkrec --interval 1000 --exit-on-end"
Enter
Sleep 30s
```

`--exit-on-end` は最終盤面と結果を 3 秒間表示してから終了するので，録画がきれいに終わります．録画しながら手動で 1 手ずつ進めたい場合は，代わりに `--paused` を使ってください．
