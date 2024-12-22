use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::TSystemCategory;
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItem};
use crate::carg::v2::{EProcessOutput, ProcessItemCreateSetting, ProcessItemCreateSettingSystem, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, TProcess, TProcessItem, TProcessItemPtr};
use crate::nz_define_time_tick_for;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use crate::wave::sample::UniformedSample;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaResampleInfo {
    /// サンプルレートに変換
    pub to_sample_rate: usize,
    ///
    pub high_quality: bool,
}

#[derive(Debug)]
pub struct ResampleProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaResampleInfo,
    internal: InternalInfo,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for ResampleProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::BUFFER_MONO),
            OUTPUT_OUT => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(input::container_category::BUFFER_MONO_DYNAMIC),
            _ => None,
        }
    }
}

impl TSystemCategory for ResampleProcessData {}
nz_define_time_tick_for!(ResampleProcessData, true, true);

impl TProcess for ResampleProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool {
        true
    }

    fn get_common_ref(&self) -> &ProcessControlItem {
        &self.common
    }

    fn get_common_mut(&mut self) -> &mut ProcessControlItem {
        &mut self.common
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) {
        // 時間更新。またInputピンのリソース更新はしなくてもいい。
        self.common.elapsed_time = input.common.elapsed_time;
        self.common.process_input_pins();

        if self.common.is_state(EProcessState::Finished) {
            return;
        }

        if self.common.is_state(EProcessState::Stopped) {
            // inputが入っているときだけ初期化できる。
            // @todo ピンからインプットに接近して、インプットが出すFsを取得する。
            let setting = {
                let input_internal = self.common.get_input_internal(INPUT_IN).unwrap();
                let input = input_internal.buffer_mono_dynamic().unwrap();
                // もしインプットがきてなくて、Fsがセットされたなきゃなんもしない。
                if input.sample_rate == 0 {
                    return;
                }

                InitializeSetting {
                    from_fs: input.sample_rate,
                    to_fs: self.info.to_sample_rate,
                    is_high_quality: self.info.high_quality,
                }
            };

            // 初期化する。
            self.initialize(&setting);
            self.common.set_state(EProcessState::Playing);
        }

        // 24-12-22 まずoverlappingなしでやってみる。
        let to_fs = self.info.to_sample_rate;
        let (input_buffer, is_last) = self.drain_buffer(&input);
        let output_buffer = self.process_resample(&input_buffer, is_last);

        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(output_buffer, to_fs)),
            )
            .unwrap();

        // 状態確認
        if is_last {
            self.common.state = EProcessState::Playing;
        } else {
            self.common.state = EProcessState::Finished;
        }
    }
}

impl TProcessItem for ResampleProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        _system_setting: &ProcessItemCreateSettingSystem,
    ) -> anyhow::Result<TProcessItemPtr> {
        if let ENode::AdapterResample(v) = setting.node {
            let item = Self {
                setting: setting.setting.clone(),
                common: ProcessControlItem::new(ENodeSpecifier::AdapterResample),
                info: v.clone(),
                internal: Default::default(),
            };

            return Ok(SItemSPtr::new(item));
        }
        unreachable!("Unexpected branch");
    }
}

impl ResampleProcessData {
    /// ノードの最初の初期化を行う。
    fn initialize(&mut self, setting: &InitializeSetting) {
        self.internal.from_fs = Some(setting.from_fs);
        self.internal.data = Some(ProcessHeader::new(setting.is_high_quality, setting.from_fs, setting.to_fs))
    }

    fn drain_buffer(&mut self, in_input: &ProcessProcessorInput) -> (Vec<UniformedSample>, bool) {
        // 24-12-22 まずソースのサンプルサイズ（1チャンネル）は固定にしてみる。
        const SRC_SAMPLE_LEN: usize = 4096;

        let mut input_internal = self
            .common
            .get_input_internal_mut(INPUT_IN)
            .unwrap();
        let input = input_internal
            .buffer_mono_dynamic_mut()
            .unwrap();

        // バッファ0補充分岐
        let is_buffer_enough = input.buffer.len() >= SRC_SAMPLE_LEN;
        if !is_buffer_enough && in_input.is_children_all_finished() {
            let mut buffer = input.buffer.drain(..).collect_vec();
            buffer.resize(SRC_SAMPLE_LEN, UniformedSample::MIN);
            return (buffer, true);
        }

        // 普通。
        (input.buffer.drain(..SRC_SAMPLE_LEN).collect_vec(), false)
    }

    fn process_resample(&self, src_buffer: &[UniformedSample], is_last: bool) -> Vec<UniformedSample> {
        // Account for increased filter gain when using factors less than 1.
        // Decimationするなら、ゲインを減らす必要があるっぽい？
        let header = self.internal.data.as_ref().unwrap();
        let lp_scale = header.ratio().min(1.0);

        //loop {
            let setting = ProcessSamplingSetting {
                src_buffer,
                ratio: header.ratio(),
                wing_num: header.wing_coeffs.len(),
                lp_scale,
                coeffs: &header.wing_coeffs,
                coeff_deltas: &header.wing_coeffs,
                use_interp: false,
                start_time: 0.0,
            };

            let result = process_source(&setting);
            result.outputs
        //}
    }
}

#[derive(Debug)]
struct ProcessSamplingSetting<'a> {
    src_buffer: &'a [UniformedSample],
    /// 1より大きいと、アップサンプリングする。
    /// 1より小さいと、ダウンサンプリングする。アップよりは少し処理負荷が高い。
    /// 1と同じであれば、何もしない。
    ratio: f64,
    wing_num: usize,
    lp_scale: f64,
    coeffs: &'a [f64],
    coeff_deltas: &'a [f64],
    use_interp: bool,
    start_time: f64,
}

#[derive(Debug)]
struct ProcessSourceResult {
    outputs: Vec<UniformedSample>,
    next_start_time: f64,
}

/// 結果の
fn process_source(setting: &ProcessSamplingSetting) -> ProcessSourceResult {
    debug_assert!(setting.ratio > 0.0);

    // Output sampling period
    let dt = setting.ratio.recip();
    let mut results = vec![];
    let mut process_time = setting.start_time;

    if setting.ratio == 1.0 {
        // そのまま
        return ProcessSourceResult {
            outputs: setting.src_buffer.iter().map(|v| *v).collect_vec(),
            next_start_time: setting.start_time + setting.src_buffer.len() as f64,
        };
    } else if setting.ratio > 1.0 {
        for sample_i in 0..setting.src_buffer.len() {
            // Interpolation
            let left_phase_frac = process_time.fract();
            let right_phase_frac = 1.0 - left_phase_frac;

            let mut proc_setting = ProcessFilterSetting {
                irs: &setting.coeffs,
                ir_deltas: &setting.coeff_deltas,
                wing_num: setting.wing_num,
                use_interp: setting.use_interp,
                phase: left_phase_frac,
                samples: &setting.src_buffer,
                start_sample_index: sample_i,
                is_increment: false,
                dh: 0.0,
            };
            // 今ターゲットになっているサンプルから左、そして右の隣接したサンプルを使って
            // 補完したサンプルを入れる。
            let mut v = 0.0;
            v += process_filter_up(&proc_setting);

            proc_setting.is_increment = true;
            proc_setting.phase = right_phase_frac;
            v += process_filter_up(&proc_setting);
            v *= setting.lp_scale;

            results.push(UniformedSample::from_f64(v));
            process_time += dt;
        }
    } else {
        // Decimation
        let npc_f = ProcessHeader::NPC as f64;
        let dh = npc_f.min(setting.ratio * npc_f);
        for sample_i in 0..setting.src_buffer.len() {
            // Interpolation
            let left_phase_frac = process_time.fract();
            let right_phase_frac = 1.0 - left_phase_frac;

            let mut proc_setting = ProcessFilterSetting {
                irs: &setting.coeffs,
                ir_deltas: &setting.coeff_deltas,
                wing_num: setting.wing_num,
                use_interp: setting.use_interp,
                phase: left_phase_frac,
                samples: &setting.src_buffer,
                start_sample_index: sample_i,
                is_increment: false,
                dh,
            };
            // 今ターゲットになっているサンプルから左、そして右の隣接したサンプルを使って
            // 補完したサンプルを入れる。
            let mut v = 0.0;
            v += process_filter_down(&proc_setting);

            proc_setting.is_increment = true;
            proc_setting.phase = right_phase_frac;
            v += process_filter_down(&proc_setting);
            v *= setting.lp_scale;

            results.push(UniformedSample::from_f64(v));
            process_time += dt;
        }
    }

    ProcessSourceResult {
        outputs: results,
        next_start_time: process_time,
    }
}

/// [`process_filter_up`]関数の設定
#[derive(Debug)]
struct ProcessFilterSetting<'a> {
    irs: &'a [f64],
    ir_deltas: &'a [f64],
    wing_num: usize,
    use_interp: bool,
    /// かならず`[0, 1]`の値を持つ。
    phase: f64,
    /// 処理内部で補完作業が必要なので、Sliceで渡すのことが必要。
    samples: &'a [UniformedSample],
    start_sample_index: usize,
    /// trueなら右側の羽を、falseなら左側の羽を。
    is_increment: bool,
    /// Decimationするときだけ使う。
    dh: f64,
}

/// `setting`の`start_sample_index`をアップスケーリングする。
fn process_filter_up(setting: &ProcessFilterSetting) -> f64 {
    debug_assert!(setting.irs.len() > 0);
    debug_assert!(setting.wing_num > 0);

    // [0, NPC)までの範囲を持つ。
    let phase_raw_i = setting.phase * ProcessHeader::NPC as f64;
    let phase_i = phase_raw_i.floor() as usize;

    // `setting::irs`、`setting:irs_delta`はNPC分を何個も持っているので、
    // 元コードではポインターを操作したけど、ここではirsとdeltaに接近するためのインデックスを操作する。
    let mut irs_i = phase_i;
    let mut irs_end_i = setting.wing_num;
    let mut phase_frac = 0.0;
    // 補完するとときだけ使う。
    if setting.use_interp {
        phase_frac = phase_raw_i - phase_i as f64;
    }

    let irs = setting.irs;
    let irs_deltas = setting.ir_deltas;
    if setting.is_increment {
        irs_end_i -= 1;
        if phase_raw_i == 0.0 {
            irs_i += ProcessHeader::NPC;
        }
    }

    let mut output = 0.0;
    let mut input_i = setting.start_sample_index;
    let inputs = setting.samples;
    if setting.use_interp {
        // irs_iを進めて、最後まで到達するまで演算する。
        loop {
            if irs_i >= irs_end_i {
                break;
            }

            let coeff = irs[irs_i] + (irs_deltas[irs_i] * phase_frac);
            let applied = coeff * inputs[input_i].to_f64();
            output += applied;

            // sinc関数を近接しているサンプルに当てるように調整する。
            irs_i += ProcessHeader::NPC;
            if setting.is_increment {
                input_i += 1;
            } else {
                input_i -= 1;
            }
        }
    } else {
        loop {
            if irs_i >= irs_end_i {
                break;
            }

            let applied = irs[irs_i] * inputs[input_i].to_f64();
            output += applied;

            // sinc関数を近接しているサンプルに当てるように調整する。
            irs_i += ProcessHeader::NPC;
            if setting.is_increment {
                input_i += 1;
            } else {
                input_i -= 1;
            }
        }
    }

    output
}

/// `setting`の`start_sample_index`をアップスケーリングする。
fn process_filter_down(setting: &ProcessFilterSetting) -> f64 {
    debug_assert!(setting.irs.len() > 0);
    debug_assert!(setting.wing_num > 0);

    // [0, NPC)までの範囲を持つ。
    let phase_raw_i = setting.phase * setting.dh;

    // `setting::irs`、`setting:irs_delta`はNPC分を何個も持っているので、
    // 元コードではポインターを操作したけど、ここではirsとdeltaに接近するためのインデックスを操作する。
    let mut irs_raw_i = phase_raw_i;
    let mut irs_i = irs_raw_i.floor() as usize;
    let mut irs_end_i = setting.wing_num;

    let irs = setting.irs;
    let irs_deltas = setting.ir_deltas;
    if setting.is_increment {
        irs_end_i -= 1;
        if setting.phase == 0.0 {
            irs_raw_i += setting.dh;
            irs_i = irs_raw_i.floor() as usize;
        }
    }

    let mut output = 0.0;
    let mut input_i = setting.start_sample_index;
    let inputs = setting.samples;
    if setting.use_interp {
        // irs_iを進めて、最後まで到達するまで演算する。
        loop {
            if irs_i >= irs_end_i {
                break;
            }

            let phase_frac = irs_raw_i - irs_i as f64;
            let coeff = irs[irs_i] + (irs_deltas[irs_i] * phase_frac);
            let applied = coeff * inputs[input_i].to_f64();
            output += applied;

            // sinc関数を近接しているサンプルに当てるように調整する。
            irs_raw_i += setting.dh;
            irs_i = irs_raw_i.floor() as usize;
            if setting.is_increment {
                input_i += 1;
            } else {
                input_i -= 1;
            }
        }
    } else {
        loop {
            if irs_i >= irs_end_i {
                break;
            }

            let applied = irs[irs_i] * inputs[input_i].to_f64();
            output += applied;

            // sinc関数を近接しているサンプルに当てるように調整する。
            irs_raw_i += setting.dh;
            irs_i = irs_raw_i.floor() as usize;
            if setting.is_increment {
                input_i += 1;
            } else {
                input_i -= 1;
            }
        }
    }

    output
}

/// 初期化の設定
#[derive(Debug, Clone)]
struct InitializeSetting {
    /// 変換のための前音波のサンプリング周波数。
    from_fs: usize,
    /// 変換後の音波のサンプリング周波数。
    to_fs: usize,
    /// 処理のクォリティーがいいか
    is_high_quality: bool,
}

// ----------------------------------------------------------------------------
// InternalInfo
// ----------------------------------------------------------------------------

#[derive(Debug)]
struct InternalInfo {
    /// サンプリング周波数は固定にする必要ある。
    /// 変動するものはサポートできない。(ADPCM?）
    from_fs: Option<usize>,
    data: Option<ProcessHeader>,
}

impl Default for InternalInfo {
    fn default() -> Self {
        Self {
            from_fs: None,
            data: None,
        }
    }
}

// ----------------------------------------------------------------------------
// ProcessHeader
// ----------------------------------------------------------------------------

#[derive(Debug)]
struct ProcessHeader {
    from_fs: usize,
    to_fs: usize,
    is_high_quality: bool,
    wing_coeffs: Vec<f64>,
    coeff_deltas: Vec<f64>,
}

impl ProcessHeader {
    const NPC: usize = 4096;

    /// サンプリング周波数の変換比率を求める。
    fn ratio(&self) -> f64 {
        self.to_fs as f64 / self.from_fs as f64
    }

    /// `from_fs`から`to_fs`までのリファクタリング処理を行うためのヘッダー情報を作る。
    /// `is_high_quality`が`true`なら、クォリティーの良いヘッダーを生成する。
    fn new(is_high_quality: bool, from_fs: usize, to_fs: usize) -> Self {
        // とりあえずlibresampleのコードを参考にしながら作ろうか。。。
        let n_mult = if is_high_quality { 35 } else { 11 } as usize;

        // wing_numは疑似sinc関数の片側の係数の数を示す。
        let wing_num = (Self::NPC * (n_mult - 1)) >> 1;
        let rolloff = 0.90;
        let beta = 6.0; // Kaiser窓関数のパラメータ

        // 片側の係数。
        // そしてそれぞれのcoeffから差分もリストに入れる。
        let wing_coeffs = Self::initialize_lpf_coeffs(wing_num, rolloff * 0.5, beta);
        let mut coeff_deltas = wing_coeffs
            .iter()
            .zip(wing_coeffs.iter().skip(1))
            .map(|(prev, next)| next - prev)
            .collect_vec();
        assert_eq!(coeff_deltas.len(), wing_num - 1);
        // 最後はcoeffsから。
        coeff_deltas.push(*wing_coeffs.last().unwrap() * -1.0);

        // LPFフィルターの片側の到達範囲？を求める？
        let ratio = (to_fs as f64) / (from_fs as f64);
        let i_ratio = ratio.recip();
        let x_offset = (((wing_num + 1) >> 1) as f64 * 1.0_f64.max(i_ratio)).floor() as usize + 10;

        Self {
            from_fs,
            to_fs,
            is_high_quality,
            wing_coeffs,
            coeff_deltas,
        }
    }

    fn initialize_lpf_coeffs(coeff_num: usize, freq: f64, beta: f64) -> Vec<f64> {
        assert!(coeff_num > 1);

        // まず窓関数を考慮しなかった、理想的なLPFフィルターの係数を入れる。
        let mut coeffs = vec![0.0; coeff_num];
        coeffs[0] = 2.0 * freq;
        for coeff_i in 1..coeff_num {
            // ここは一般sinc関数を使わない。
            let v = PI * coeff_i as f64 / (coeff_num as f64);
            coeffs[coeff_i] = (2.0 * v * freq).sin() / v;
        }

        // カイザー窓を適用する。
        // https://en.wikipedia.org/wiki/Kaiser_window
        let ibeta = Self::modified_bessel_1st(beta).recip();
        let inm1 = ((coeff_num - 1) as f64).recip();
        for coeff_i in 1..coeff_num {
            let v = (coeff_i as f64) * inm1; // [0, 1]。(2x/L)の部分
            let v = 1.0 - (v * v); // 1 - (2x/L)^2の部分
            let v = v.max(0.0); // sqrtするので、マイナスは許容できない。

            // ここで値をベッセル関数に入れて補正する。
            coeffs[coeff_i] *= Self::modified_bessel_1st(beta * v.sqrt()) * ibeta;
        }

        coeffs
    }

    /// https://en.wikipedia.org/wiki/Bessel_function#Modified_Bessel_functions:_I%CE%B1,_K%CE%B1
    /// を参考すること。
    fn modified_bessel_1st(x: f64) -> f64 {
        let mut sum = 1.0;
        let half_x = x * 0.5;
        let mut n = 1.0;
        let mut u = 1.0;

        loop {
            let mut temp = half_x / n;
            n += 1.0;

            temp *= temp;
            u *= temp;
            sum += u;

            if u < (f64::EPSILON * sum) {
                break;
            }
        }

        sum
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
