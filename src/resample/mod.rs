use crate::wave::sample::UniformedSample;
use itertools::Itertools;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::{Arc, Mutex, OnceLock, Weak};

/// 24-12-23
/// リサンプリングするためのテンプレートや接近するための仕組みを用意している。
///
/// たとえば441⇔480はよく使われるなどで、そのためのIR係数をあらかじめ保存して
/// さまざまな所で使いまわせることで色々なメリットがある。
static RESAMPLE_SYSTEM: OnceLock<Arc<Mutex<ResampleSystem>>> = OnceLock::new();

/// WeakPtrなので解放はしなくてもいいかもしれない。
static PROXY_ACCESSOR: OnceLock<ResampleSystemProxyWeakPtr> = OnceLock::new();

pub struct ResampleSystem {
    v: Option<ResampleSystemInternal>,
}

impl ResampleSystem {
    pub fn initialize(config: ResampleSystemConfig) -> ResampleSystemProxyWeakPtr {
        let original_proxy = {
            assert!(RESAMPLE_SYSTEM.get().is_none());

            let _result = RESAMPLE_SYSTEM.set(Arc::new(Mutex::new(Self::new(config))));
            let system = RESAMPLE_SYSTEM.get().unwrap();
            let weak = Arc::downgrade(&system);

            let original_proxy = ResampleSystemProxy::new(weak);
            original_proxy
        };

        // Proxyの登録
        let weak_proxy = Arc::downgrade(&original_proxy);
        {
            // Mutexがおそらく内部Internal Mutabilityを実装しているかと。
            let instance = RESAMPLE_SYSTEM.get().expect("ResampleSystem instance must be valid");
            let mut accessor = instance.lock().unwrap();
            debug_assert!(accessor.v.is_some());

            let v = accessor.v.as_mut().unwrap();
            v.original_proxy = Some(original_proxy);
        }

        // Proxyを返す。本体は絶対返さない。
        assert!(RESAMPLE_SYSTEM.get().is_some());

        let _result = PROXY_ACCESSOR.set(weak_proxy.clone());
        weak_proxy
    }

    fn new(config: ResampleSystemConfig) -> Self {
        Self {
            v: Some(ResampleSystemInternal::new(config)),
        }
    }

    /// システムを解放する。
    /// すべての関連処理が終わった後に解放すべき。
    pub fn cleanup() {
        assert!(RESAMPLE_SYSTEM.get().is_some());

        if let Some(system) = RESAMPLE_SYSTEM.get() {
            let mut system = system.lock().unwrap();
            system.v = None;
        }
    }

    /// `setting`に合わせてリサンプリングのための新しいIRを生成する。
    fn create_response(&mut self, setting: &ResampleHeaderSetting) {
        let v = self.v.as_mut().unwrap();

        if !v.map.contains_key(&setting) {
            let header = setting.create_header();
            v.map.insert(setting.clone(), header);
        }
    }

    /// `ir_setting`から取得したリサンプリングのIRに`buffer_setting`を適用してリサンプリングする。
    fn process_response(
        &self,
        ir_setting: &ResampleHeaderSetting,
        buffer_setting: &ProcessSamplingSetting,
    ) -> anyhow::Result<ProcessSourceResult> {
        let v = self.v.as_ref().unwrap();

        match v.map.get(&ir_setting) {
            None => Err(anyhow::anyhow!("Given ir_setting {:?} is not created yet.", ir_setting)),
            Some(v) => Ok(v.process(&buffer_setting))
        }
    }
}

/// システム立ち上げの初期設定
#[derive(Debug, Clone)]
pub struct ResampleSystemConfig {}

impl ResampleSystemConfig {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct ResampleSystemProxy {
    device: Weak<Mutex<ResampleSystem>>,
}

impl ResampleSystemProxy {
    fn new(device: Weak<Mutex<ResampleSystem>>) -> ResampleSystemProxyPtr {
        let instance = Self { device };
        Arc::new(Mutex::new(instance))
    }

    /// `setting`に合わせてリサンプリングのための新しいIRを生成する。
    pub fn create_response(&mut self, setting: &ResampleHeaderSetting) {
        match self.device.upgrade() {
            None => (),
            Some(v) => {
                let mut v = v.lock().unwrap();
                v.create_response(&setting);
            }
        }
    }

    /// `ir_setting`から取得したリサンプリングのIRに`buffer_setting`を適用してリサンプリングする。
    pub fn process_response(
        &self,
        ir_setting: &ResampleHeaderSetting,
        buffer_setting: &ProcessSamplingSetting,
    ) -> anyhow::Result<ProcessSourceResult> {
        match self.device.upgrade() {
            None => Err(anyhow::anyhow!("System is not setup.")),
            Some(v) => {
                let v = v.lock().unwrap();
                v.process_response(&ir_setting, &buffer_setting)
            }
        }
    }
}

type ResampleSystemProxyPtr = Arc<Mutex<ResampleSystemProxy>>;
pub type ResampleSystemProxyWeakPtr = Weak<Mutex<ResampleSystemProxy>>;

// ----------------------------------------------------------------------------
// 内部制御
// ----------------------------------------------------------------------------

struct ResampleSystemInternal {
    /// リサンプリングのマップの保持情報
    map: HashMap<ResampleHeaderSetting, ResampleProcessHeader>,
    /// プロキシの親元。ほかのところでは全部Weakタイプで共有する。
    original_proxy: Option<ResampleSystemProxyPtr>,
    /// 初期設定
    #[allow(dead_code)]
    initial_config: ResampleSystemConfig,
}

impl ResampleSystemInternal {
    fn new(config: ResampleSystemConfig) -> Self {
        Self {
            map: HashMap::new(),
            original_proxy: None,
            initial_config: config.clone(),
        }
    }
}

// ----------------------------------------------------------------------------
// 関連タイプ
// ----------------------------------------------------------------------------

/// 特性関数の理想的なsinc関数を考慮する時の、per zero-crossing間のサンプル数を示す。
const NPC: usize = 4096;

/// リサンプリングのヘッダーのアイテム
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResampleHeaderSetting {
    /// サンプリングレートの変換前
    pub from_fs: usize,
    /// サンプリングレートの変換後
    pub to_fs: usize,
    /// フィルタのタップ数が多いか
    pub is_high_quality: bool,
}

impl ResampleHeaderSetting {
    pub fn create_header(&self) -> ResampleProcessHeader {
        ResampleProcessHeader::new(&self)
    }

    pub fn ratio(&self) -> f64 {
        self.to_fs as f64 / self.from_fs as f64
    }
}

/// フィルターの係数の情報
#[derive(Debug)]
pub struct ResampleProcessHeader {
    info: ResampleHeaderSetting,
    wing_num: usize,
    wing_coeffs: Vec<f64>,
    coeff_deltas: Vec<f64>,
}

impl ResampleProcessHeader {
    /// `from_fs`から`to_fs`までのリファクタリング処理を行うためのヘッダー情報を作る。
    /// `is_high_quality`が`true`なら、クォリティーの良いヘッダーを生成する。
    fn new(setting: &ResampleHeaderSetting) -> Self {
        // とりあえずlibresampleのコードを参考にしながら作ろうか。。
        // libresampleのヘッダーではインプットのバッファを入れるためのX系列の変数もあるけど、
        // x_offが前の分のオフセットで、x_readから～が新しいインプットを入れるためのバッファ。
        let is_high_quality = setting.is_high_quality;

        // 偶数にすること。
        // 半分はsinc関数の各羽部分にあたる。
        let n_mult = if is_high_quality { 35 } else { 11 } as usize;
        assert_eq!(n_mult % 2, 1);

        // wing_numは疑似sinc関数の片側の係数の数を示す。
        // つまり、n_multの半分のzero-crossingが存在するともいえる。
        let wing_num = (NPC * (n_mult - 1)) >> 1;
        let rolloff = 0.5;
        let beta = PI * 2.0; // Kaiser窓関数のパラメータ beta = pi*alpha.

        // 片側の係数。
        // そしてそれぞれのcoeffから差分もリストに入れる。
        let wing_coeffs = initialize_lpf_coeffs(wing_num, rolloff, beta, NPC);
        let mut coeff_deltas = wing_coeffs
            .iter()
            .zip(wing_coeffs.iter().skip(1))
            .map(|(prev, next)| next - prev)
            .collect_vec();
        assert_eq!(coeff_deltas.len(), wing_num - 1);
        // 最後はcoeffsから。
        coeff_deltas.push(*wing_coeffs.last().unwrap() * -1.0);

        Self {
            info: setting.clone(),
            wing_num,
            wing_coeffs,
            coeff_deltas,
        }
    }

    pub fn process(&self, input: &ProcessSamplingSetting) -> ProcessSourceResult {
        debug_assert!(input.start_phase_time >= 0.0 && input.start_phase_time <= 1.0);
        debug_assert!(input.process_length > 0);
        debug_assert!(input.start_sample_i + input.process_length <= input.src_buffer.len());

        // Output sampling period
        //
        // 元コードでは`dt`だったが、今のinputバッファから何サンプル分たっているか？を示す。
        // 24kHzから48kHzなら、ratio = 2でdt = 0.5だけどつまり1サンプルに2個分計算する。
        // という意味にもなる。
        let ratio = self.info.ratio();
        let buffer_proceed_delta = ratio.recip();
        let input_buffer_end_i = input.process_length + input.start_sample_i;

        // 24-12-22 Phaseを計算するために必要。サンプルをとるための時間計算は今は別途する。
        let mut results = vec![];
        let mut phase_time: f64 = input.start_phase_time;

        if ratio == 1.0 {
            // そのまま
            return ProcessSourceResult {
                outputs: input.src_buffer.iter().map(|v| *v).collect_vec(),
                next_phase_time: (input.start_phase_time + input.src_buffer.len() as f64).recip(),
            };
        } else if ratio > 1.0 {
            let mut proc_setting = ProcessFilterSetting {
                irs: &self.wing_coeffs,
                ir_deltas: &self.coeff_deltas,
                wing_num: self.wing_num,
                use_interp: input.use_interp,
                phase: 0.0,
                samples: &input.src_buffer,
                start_sample_index: 0,
                is_increment: false,
                dh: 0.0,
            };

            // Interpolation
            loop {
                let input_i: usize = (phase_time.floor() as usize) + input.start_sample_i;
                if input_i >= input_buffer_end_i {
                    break;
                }
                proc_setting.start_sample_index = input_i;

                // これがずれてしまうと、Aliasingが起きてしまう。
                // phase_fracは生成するサンプルが前のサンプルと後のサンプルの間の位置で
                // leftに移動するときにはそのまま。（sincの同じ側）
                // rightに移動するときには逆。（sincの反対側にそう）
                let left_phase_frac = phase_time.fract();
                let right_phase_frac = 1.0 - left_phase_frac;

                // 今ターゲットになっているサンプルから左、そして右の隣接したサンプルを使って
                // 補完したサンプルを入れる。
                let mut v = 0.0;

                proc_setting.is_increment = false;
                proc_setting.phase = left_phase_frac;
                v += process_filter_up(&proc_setting);

                proc_setting.is_increment = true;
                proc_setting.phase = right_phase_frac;
                v += process_filter_up(&proc_setting);

                results.push(UniformedSample::from_f64(v));
                phase_time += buffer_proceed_delta;
            }
        } else {
            // Decimation
            let npc_f = NPC as f64;
            let dh = npc_f.min(ratio * npc_f);
            let amplitude_scale = ratio.min(1.0);

            loop {
                let input_i: usize = (phase_time.floor() as usize) + input.start_sample_i;
                if input_i >= input_buffer_end_i {
                    break;
                }

                // Interpolation
                let left_phase_frac = phase_time.fract();
                let right_phase_frac = 1.0 - left_phase_frac;

                let mut proc_setting = ProcessFilterSetting {
                    irs: &self.wing_coeffs,
                    ir_deltas: &self.coeff_deltas,
                    wing_num: self.wing_num,
                    use_interp: input.use_interp,
                    phase: left_phase_frac,
                    samples: &input.src_buffer,
                    start_sample_index: input_i,
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
                v *= amplitude_scale;

                results.push(UniformedSample::from_f64(v));
                phase_time += buffer_proceed_delta;
            }
        }

        ProcessSourceResult {
            outputs: results,
            next_phase_time: phase_time.fract(),
        }
    }
}

#[derive(Debug)]
pub struct ProcessSamplingSetting<'a> {
    pub src_buffer: &'a [UniformedSample],
    pub use_interp: bool,
    pub start_phase_time: f64,
    pub start_sample_i: usize,
    pub process_length: usize,
}

/// [`process_source`]の処理結果
#[derive(Debug)]
pub struct ProcessSourceResult {
    /// 処理後のサンプルリスト
    pub outputs: Vec<UniformedSample>,
    /// 次のバッファを処理する時のPhaseを計算するための時間。
    pub next_phase_time: f64,
}

// ----------------------------------------------------------------------------
// 補助関数
// ----------------------------------------------------------------------------

pub fn initialize_lpf_coeffs(coeff_num: usize, _rolloff: f64, beta: f64, zero_crossing: usize) -> Vec<f64> {
    assert!(coeff_num > 1);

    // まず窓関数を考慮しなかった、理想的なLPFフィルターの係数を入れる。
    // ただこれだけじゃAliasingがおきるので、次にカイザー窓で抑えさえる。
    let mut coeffs = vec![0.0; coeff_num];
    let local_num = (zero_crossing as f64).recip();
    coeffs[0] = 1.0;
    for coeff_i in 1..coeff_num {
        // ここは一般sinc関数を使わない。
        let v = PI * coeff_i as f64 * local_num;
        let v_recip = v.recip();
        coeffs[coeff_i] = v.sin() * v_recip;
    }

    // カイザー窓を適用する。
    // https://en.wikipedia.org/wiki/Kaiser_window
    let beta_recip = modified_bessel_1st(beta).recip();
    let inm1 = (coeff_num as f64).recip();
    for coeff_i in 1..coeff_num {
        let v = (2.0 * coeff_i as f64) * inm1; // [0, 1]。(2x/L)の部分
        let v = 1.0 - (v * v); // 1 - (2x/L)^2の部分
        let v = v.max(0.0); // sqrtするので、マイナスは許容できない。

        // ここで値をベッセル関数に入れて補正する。
        // mul値自体は[0, 1]を持つ。
        let mul = modified_bessel_1st(beta * v.sqrt()) * beta_recip;
        debug_assert!(mul >= 0.0);
        coeffs[coeff_i] *= mul;
    }

    coeffs
}

/// https://en.wikipedia.org/wiki/Bessel_function#Modified_Bessel_functions:_I%CE%B1,_K%CE%B1
/// を参考すること。
///
/// alphaは0になので、基本1.0から始まる。
fn modified_bessel_1st(x: f64) -> f64 {
    let mut sum = 1.0;
    let half_x = x * 0.5;
    let mut u = 1.0;

    for n in 1usize.. {
        // pow2
        let mut temp = half_x / (n as f64);
        temp *= temp;

        u *= temp;
        sum += u;

        if u < (f64::EPSILON * sum) {
            break;
        }
    }

    sum
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
    debug_assert!(setting.phase >= 0.0 && setting.phase <= 1.0);

    // [0, NPC)までの範囲を持つ。
    // 生成されるサンプルの位置を決めて、それにあわせてsincのIRから特定の位置（NPCの間）の値を反映する。
    //
    // `setting::irs`、`setting:irs_delta`はNPC分を何個も持っているので、
    // 元コードではポインターを操作したけど、ここではirsとdeltaに接近するためのインデックスを操作する。
    let phase_raw_i = setting.phase * NPC as f64;
    let mut phase_i = phase_raw_i.floor() as usize;

    let mut irs_i = phase_i;
    let irs_end_i = setting.wing_num;
    let mut phase_frac = 0.0;
    // 補完するとときだけ使う。
    if setting.use_interp {
        phase_frac = phase_raw_i - phase_i as f64;
        debug_assert!(phase_frac >= 0.0);
    }

    let irs = setting.irs;
    let irs_deltas = setting.ir_deltas;

    let mut output = 0.0;
    let mut input_i = setting.start_sample_index;
    let inputs = setting.samples;

    // irs_iを進めて、最後まで到達するまで演算する。
    loop {
        // サンプルを補完するためのIRがなければ、終わる。(FIRなのでタップの限界がある)
        // 35個か、11個か。
        if irs_i >= irs_end_i {
            break;
        }

        let coeff = if setting.use_interp {
            irs[irs_i] + (irs_deltas[irs_i] * phase_frac)
        } else {
            irs[irs_i]
        };

        output += coeff * inputs[input_i].to_f64();

        // sinc関数を近接しているサンプルに当てるように調整する。
        // 片側のzero-crossingの中の区間がNPC個のサンプルが入っているとしたら、
        // 次のサンプルに当てはまるIR係数はNPC先である。
        irs_i += NPC;

        if setting.is_increment {
            input_i += 1;
        } else {
            input_i -= 1;
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
    let mut is_oob = false;
    let mut last_sample = 0.0;
    let inputs = setting.samples;
    loop {
        // サンプルを補完するためのIRがなければ、終わる。(FIRなのでタップの限界がある)
        if irs_i >= irs_end_i {
            break;
        }

        let coeff = if setting.use_interp {
            let phase_frac = irs_raw_i - irs_i as f64;
            irs[irs_i] + (irs_deltas[irs_i] * phase_frac)
        } else {
            irs[irs_i]
        };

        let input = if is_oob { last_sample } else { inputs[input_i].to_f64() };
        last_sample = input; // oobした時にこれを使う。
        let applied = coeff * input;
        output += applied;

        // sinc関数を近接しているサンプルに当てるように調整する。
        irs_raw_i += setting.dh;
        irs_i = irs_raw_i.floor() as usize;
        if is_oob {
            continue;
        }

        if setting.is_increment {
            input_i += 1;

            if input_i >= inputs.len() {
                is_oob = true;
            }
        } else {
            if input_i > 0 {
                input_i -= 1;
            } else {
                is_oob = true;
            }
        }
    }

    output
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
