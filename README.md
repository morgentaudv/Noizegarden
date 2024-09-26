# Noizegarden

> 前FourierPractice (Formerly, FourierPractice)

Rust言語で作成中の、ノードベースのプロシージャルの音型生成、音型分析プログラムです。

Node based musical analysis application written in Rustlang, also features making procedural sound waves, and so on.

## Todo List

* FFT分析、IFFT変換ノードの実装
* Delta Timeのモードの反映
* LU測定ノード
* LPFノード
* HPFノード
* リアルタイムプレビューノード

---

# FourierPractice

「サウンドプログラミング入門（青木　直史、著）」を読みながら内容をRustで実装したロジックの塊です。

## 実行方法

このプログラムはメインロジックを使いません。代わりにcargoのtestを使ってそれぞれのロジックを実行します。

```
cargo test
```

`/assets/`フォルダーの各チャプターフォルダーの中に、各ロジックから出力したwavファイルが確認できます。
