# Noizegarden

> 前FourierPractice (Formerly, FourierPractice)

Rust言語で作成中の、ノードベースのプロシージャルの音型生成、音型分析プログラムです。
1次的に

Node based musical analysis application written in Rust-lang, also features making procedural sound waves, and so on.

# Example（サンプル設定）の実行方法

まず`Noizegarden`を作るためには、最新の`rust`や`cargo`をインストールして以下のコマンドを実行します。

```
cargo build --release
```

`example`フォルダーにある、サンプルの設定を読み込んで音源の発生処理を行うには、生成したファイルから

```
noizegarden -i ./example/sine_sweep.json
```

のように読み込ませたいファイルのパスを入力して実行します。

# Examples

`./example`フォルダーに音波の処理を行う設定ファイルがあります。
これを起動してみることで、処理を構成しているノードの構成がわかります。

ここでは各ファイルの概要だけを紹介いたします。

| File | Description                            |
| --- |----------------------------------------|
| sine.json | A4(440Hz)のサイン波形を3秒間発生したmonoの音源を出力します。  | 
| square.json | A4(440Hz)の矩形波を3秒発生したmonoの音源を出力します。 |
| sawtooth.json | A4(440Hz)のノコギリ波を3秒発生したmonoの音源を出力します。 |
| triangle.json | A4(440Hz)の三角波を3秒発生したmonoの音源を出力します。 |
| whitenoise.json | ホワイトノイズを3秒発生したmonoの音源を出力します。 |
| pinknoise.json | ピンクノイズを3秒発生したmonoの音源を出力します。 |
| sweep.json | サインスイープを20Hzから20000Hzまで発生したmonoの音源を出力します。 |


---

# 目標

* (最優先) PureData/SuperColliderのような音響合成プログラムを目指す
* ゲームと連携してStarveなく音が流れるような仕組みにする。
* エディターツールを作り、音響合成の作成をやりやすくする。(egui, webgpu)

# Todo List

- [x] FFT分析、IFFT変換ノードの実装
- [x] DFTの窓関数の適用
- [x] DFTの50%Overlap適用
- [x] DFTの50%Overlapがバグっているので確認して修正すること。
- [x] FFTの窓関数の適用
- [x] FFTの50%Overlap適用
- [x] 音源(wav, 16bit, mono)Emitterノードの追加
- [x] 48000kHz LUFS測定ノード
- [x] 44100kHz LUFS測定ノード
- [x] Limiterノード
- [x] Compressorノード
- [x] FIRのLPF(Edge, Delta) ノード
- [x] FIRのHPF
- [x] FIRのBPF
- [x] FIRのBEF
- [x] IIRのLPFノード
- [x] IIRのHPFノード
- [x] IIRのBPF
- [x] IIRのBSF
- [x] Resamplingの作りなおし
- [x] Resamplingのバグ対応?
- [x] sine-sweepのEmitterノード
- [x] FileIO制御のシステム化
- [ ] `sample_count_frame`の廃止と代替案の導入
- [ ] LUFSのゲーティング処理やそれに伴うIntegratedの実装
- [ ] eguiの導入
- [ ] webgpuの導入 (vulkanは難易度高すぎたため)
- [ ] 音源(wav, 16bit, stereo)Emitterノードの追加
- [ ] IRConvolutionノード
- [ ] Delta Timeのモードの反映
- [ ] リアルタイムプレビューノード
- [ ] Emitter音源発生系ノードのトリガー統合？
- [ ] ピンのアイテムプール化の検証
- [ ] 最適化

---

# FourierPractice

「サウンドプログラミング入門（青木　直史、著）」を読みながら内容をRustで実装したロジックの塊です。

## 実行方法

このプログラムはメインロジックを使いません。代わりにcargoのtestを使ってそれぞれのロジックを実行します。

```
cargo test
```

`/assets/`フォルダーの各チャプターフォルダーの中に、各ロジックから出力したwavファイルが確認できます。
