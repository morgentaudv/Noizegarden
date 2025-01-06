# Noizegarden

<img src="./example/Screenshot 2025-01-06 175759.png" width=800 alt="resample.jsonを実行した図"/>

resample.jsonを実行した図

> 前FourierPractice (Formerly, FourierPractice)

Rust言語で作成中の、ノードベースのプロシージャルの音型生成、音型分析プログラムです。
1次的に

Node based musical analysis application written in Rust-lang, also features making procedural sound waves, and so on.

## Example（サンプル設定）の実行方法

まず`Noizegarden`を作るためには、最新の`rust`や`cargo`をインストールして以下のコマンドを実行します。

```
cargo build --release
```

`example`フォルダーにある、サンプルの設定を読み込んで音源の発生処理を行うには、生成したファイルから

```
noizegarden -i ./example/sine_sweep.json
```

のように読み込ませたいファイルのパスを入力して実行します。

## Examples

`./example`フォルダーに音波の処理を行う設定ファイルがあります。
これを起動してみることで、処理を構成しているノードの構成がわかります。

ここでは各ファイルの概要だけを紹介いたします。

| File               | Description                                                  |
|--------------------|--------------------------------------------------------------|
| sine.json          | A4(440Hz)のサイン波形を3秒間発生したmonoの音源を出力します。                        | 
| square.json        | A4(440Hz)の矩形波を3秒発生したmonoの音源を出力します。                           |
| sawtooth.json      | A4(440Hz)のノコギリ波を3秒発生したmonoの音源を出力します。                         |
| triangle.json      | A4(440Hz)の三角波を3秒発生したmonoの音源を出力します。                           |
| whitenoise.json    | ホワイトノイズを3秒発生したmonoの音源を出力します。                                 |
| pinknoise.json     | ピンクノイズを3秒発生したmonoの音源を出力します。                                  |
| sweep.json         | サインスイープを20Hzから20000Hzまで発生したmonoの音源を出力します。                    |
| wav_mono.json      | 特定のmono音源を読み込み、400HzをカットオフとするLPF(FIR)をかけて出力します。              |
| dft.json           | DFT(Discrete Fourier Transform)と、逆変換を使って音源の周波数を分析して結果を出力します。 |
| fft.json           | FFT(Fast Fourier Transform)と、その逆変換を使って音源の周波数を分析、再現します。       |
| envelope_ad.json   | エンベロープ(AD)ノードを使って、音源の振幅を調整します。                               |
| envelope_adsr.json | エンベロープ(ADSR)ノードを使って、音源振幅を調整します。                              |
| lufs.json          | mono音源のLUFSを測定します。ただしゲーティング処理や音源全体のLUFSの測定は行いません。            |
| wave_sum.json      | C長調のmaj5の和音の正弦波を合成し、mono音源として出力します。                          |
| compressor.json    | compressorを使って元mono音源のレベルを抑制します。                             |
| limiter.json       | limiterを使って元mono音源のレベルを抑制します。                                |
| fir_lpf.json       | `wav_mono.json`と設定は同じです。FIRフィルターを使い、元音源から400Hz以下だけを残します。     |
| fir_hpf.json       | FIRフィルターを使い、元音源から2kHz以上の音だけを残します。                            |
| fir_bpf.json       | FIRフィルターを使い、元音源から1kHz回りの音だけを残します。                            |
| fir_bsf.json       | FIRフィルターを使い、元音源から2kHz回りを除いた音だけを残します。                         |
| iir_lpf.json       | IIRフィルター(biquad)を使い、元音源から400Hz以下だけを残します。                     |
| iir_hpf.json       | IIRフィルター(biquad)を使い、元音源から2kHz以上の音だけを残します。                    |
| iir_bpf.json       | IIRフィルター(biquad)を使い、元音源から1kHz周りの音だけを残します。                    |
| iir_bef.json       | IIRフィルター(biquad)を使い、元音源から2kHz周りを除いた音だけを残します。                 |
| mix_stereo.json    | mono音源をステレオの各チャンネルに構成します。現在パンニングの調整はできません。                   |
| resample.json      | 20hzから20kHzまで続くサインスイープを96kHzレートから48kHzに変換して出力します。            |
| delay.json         | mono音源を50msずらして流します。                                         |
| pseudo_stereo.json | Delayノードを使い、mono音源から疑似的なステレオを構築します。                          |
| ir_conv.json       | (TODO) mono音源に対しIR畳み込みを行います。                                 |

---

# 目標

* (最優先) PureData/SuperColliderのような音響合成プログラムを目指す。
* ゲームと連携してStarveなく音が流れるような仕組みにする。
* エディターツールを作り、音響合成の作成をやりやすくする。(egui, webgpu)

## Todo List

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
- [x] monoのDelayノード
- [ ] `sample_count_frame`の廃止と代替案の導入
- [ ] LUFSのゲーティング処理やそれに伴うIntegratedの実装
- [ ] eguiの導入
- [ ] webgpuの導入 (vulkanは難易度高すぎたため)
- [ ] 音源(wav, 16bit, stereo)Emitterノードの追加
- [ ] IRConvolutionノード
- [ ] Delta Timeのモードの反映
- [ ] リアルタイムプレビューノード
- [ ] Emitter音源発生系ノードのトリガー統合？
- [ ] サンプルレートが違う時に自動でサンプルレート変換処理を行う
- [ ] ピンのアイテムプール化の検証
- [ ] 最適化

---
---

# FourierPractice

「サウンドプログラミング入門（青木　直史、著）」を読みながら内容をRustで実装したロジックの塊です。

## 実行方法

このプログラムはメインロジックを使いません。代わりにcargoのtestを使ってそれぞれのロジックを実行します。

```
cargo test
```

`/assets/`フォルダーの各チャプターフォルダーの中に、各ロジックから出力したwavファイルが確認できます。
