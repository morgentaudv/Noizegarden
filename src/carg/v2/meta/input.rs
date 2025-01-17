use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::output::output_file::EOutputFileInput;
use crate::carg::v2::output::output_log::EOutputLogItem;
use crate::carg::v2::output::output_device::EOutputDeviceInput;
use crate::wave::sample::UniformedSample;

/// [`EProcessInputContainer`]の各アイテムの識別子をまとめている。
pub mod container_category {
    pub const UNINITIALIZED: u64 = 0;

    /// 空
    pub const EMPTY: u64 = 1 << 0;

    /// 動的にWaveBufferを保持するコンテナとして運用する。
    pub const BUFFER_MONO_DYNAMIC: u64 = 1 << 1;

    /// バッファを受け取るにはするが、内部処理はしない。
    pub const BUFFER_MONO_PHANTOM: u64 = 1 << 2;

    /// [`pin_category::BUFFER_STEREO`]を動的に保持するコンテナとして運用する。
    pub const BUFFER_STEREO_DYNAMIC: u64 = 1 << 3;

    /// 動的にTextのリストを保持するコンテナとして運用する。
    pub const TEXT_DYNAMIC: u64 = 1 << 4;

    /// [`pin_category::FREQUENCY`]を参照するが、Inputでなにかを持ったりはしない。
    pub const FREQUENCY_PHANTOM: u64 = 1 << 5;

    /// ダミーのインプット。なんでもあり。
    pub const DUMMY: u64 = 1 << 6;

    /// [`ENodeSpecifier::OutputFile`]専用
    pub const OUTPUT_FILE: u64 = BUFFER_MONO_DYNAMIC | BUFFER_STEREO_DYNAMIC | OUTPUT_FILE_INTERNAL_TAG;

    /// [`ENodeSpecifier::OutputLog`]専用
    pub const OUTPUT_LOG: u64 = BUFFER_MONO_DYNAMIC | TEXT_DYNAMIC;

    /// [`ENodeSpecifier::OutputDevice`]専用
    pub const OUTPUT_DEVICE: u64 = BUFFER_MONO_DYNAMIC | BUFFER_STEREO_DYNAMIC | OUTPUT_DEVICE_INTERNAL_TAG;

    // ------------------------------------------------------------------------
    // 内部用
    // ------------------------------------------------------------------------

    /// 内部識別区別タグ
    const OUTPUT_FILE_INTERNAL_TAG: u64 = 1 << 48;

    /// 内部識別区別タグ
    const OUTPUT_DEVICE_INTERNAL_TAG: u64 = 1 << 49;
}

pub type EInputContainerCategoryFlag = u64;

#[derive(Debug, Clone)]
pub enum EProcessInputContainer {
    Uninitialized,
    Empty,
    Dummy,
    BufferMonoDynamic(BufferMonoDynamicItem),
    BufferMonoPhantom,
    BufferStereoDynamic(BufferStereoDynamicItem),
    OutputFile(EOutputFileInput),
    TextDynamic(TextDynamicItem),
    OutputLog(EOutputLogItem),
    OutputDevice(EOutputDeviceInput),
    FrequencyPhantom,
}

impl EProcessInputContainer {
    /// [`container_category::BUFFER_MONO_DYNAMIC`]を指定してる時に使える。
    pub fn buffer_mono_dynamic(&self) -> Option<&BufferMonoDynamicItem> {
        if let Self::BufferMonoDynamic(item) = self {
            Some(item)
        }
        else {
            None
        }
    }

    /// [`container_category::BUFFER_MONO_DYNAMIC`]を指定してる時に使える。
    pub fn buffer_mono_dynamic_mut(&mut self) -> Option<&mut BufferMonoDynamicItem> {
        if let Self::BufferMonoDynamic(item) = self {
            Some(item)
        }
        else {
            None
        }
    }

    /// [`container_category::BUFFER_STEREO_DYNAMIC`]を指定してる時に使える。
    pub fn buffer_stereo_dynamic(&self) -> Option<&BufferStereoDynamicItem> {
        if let Self::BufferStereoDynamic(item) = self {
            Some(item)
        }
        else {
            None
        }
    }

    /// [`container_category::BUFFER_STEREO_DYNAMIC`]を指定してる時に使える。
    pub fn buffer_stereo_dynamic_mut(&mut self) -> Option<&mut BufferStereoDynamicItem> {
        if let Self::BufferStereoDynamic(item) = self {
            Some(item)
        }
        else {
            None
        }
    }

    /// [`container_category::OUTPUT_DEVICE`]を指定してる時に使える。
    pub fn output_dynamic(&self) -> Option<&EOutputDeviceInput> {
        match self {
            Self::OutputDevice(item) => Some(item),
            _ => None,
        }
    }

    /// [`container_category::OUTPUT_DEVICE`]を指定してる時に使える。
    pub fn output_dynamic_mut(&mut self) -> Option<&mut EOutputDeviceInput> {
        match self {
            Self::OutputDevice(item) => Some(item),
            _ => None,
        }
    }
}

/// [`EProcessInputContainer::BufferMonoDynamic`]の内部コンテナ
#[derive(Debug, Clone)]
pub struct BufferMonoDynamicItem {
    pub buffer: Vec<UniformedSample>,
    pub sample_rate: usize,
}

impl BufferMonoDynamicItem {
    pub fn new() -> Self {
        Self {
            buffer: vec![],
            sample_rate: 0,
        }
    }

    /// 処理可能か？
    pub fn can_process(&self) -> bool {
        self.sample_rate != 0
    }

    pub fn update(&mut self, output: &EProcessOutputContainer) {
        // WaveBufferであるかをチェック。
        match output {
            // 記入する。
            EProcessOutputContainer::BufferMono(v) => {
                self.sample_rate = v.sample_rate;

                // 24-09-27 `sample_offset`に気をつける。
                let sample_offset = v.sample_offset.min(self.buffer.len());
                if sample_offset == 0 {
                    self.buffer.append(&mut v.buffer.clone());
                }
                else {
                    // bufferのラストからsample_offset分を持ってくる。
                    {
                        let buffer_it = self.buffer.iter_mut().rev().take(sample_offset).rev();
                        let v_it = v.buffer.iter().take(sample_offset);
                        buffer_it.zip(v_it).for_each(|(dst, src)| *dst += *src);
                    }

                    // 残り分は全部Appendする。
                    self.buffer.append(&mut v.buffer.iter().skip(sample_offset).copied().collect());
                }
            }
            _ => unreachable!("Unexpected output"),
        }
    }
}

/// [`EProcessInputContainer::BufferStereoDynamic`]の内部コンテナ
#[derive(Debug, Clone)]
pub struct BufferStereoDynamicItem {
    pub ch_left: Vec<UniformedSample>,
    pub ch_right: Vec<UniformedSample>,
    pub sample_rate: usize,
}

impl BufferStereoDynamicItem {
    pub fn new() -> Self {
        Self {
            ch_left: vec![],
            ch_right: vec![],
            sample_rate: 0,
        }
    }

    /// 処理可能か？
    pub fn can_process(&self) -> bool {
        self.sample_rate != 0
    }

}

/// [`EProcessInputContainer::TextDynamic`]の内部コンテナ
#[derive(Debug, Clone)]
pub struct TextDynamicItem {
    pub buffer: Vec<String>,
}

impl TextDynamicItem {
    pub fn new() -> Self {
        Self { buffer: vec![] }
    }
}

impl EProcessInputContainer {
    /// 現在保持しているInputコンテナを識別するためのフラグを返す。
    pub fn as_container_category_flag(&self) -> EInputContainerCategoryFlag {
        match self {
            EProcessInputContainer::Uninitialized => container_category::UNINITIALIZED,
            EProcessInputContainer::BufferMonoPhantom => container_category::BUFFER_MONO_PHANTOM,
            EProcessInputContainer::Empty => container_category::EMPTY,
            EProcessInputContainer::BufferMonoDynamic(_) => container_category::BUFFER_MONO_DYNAMIC,
            EProcessInputContainer::BufferStereoDynamic(_) => container_category::BUFFER_STEREO_DYNAMIC,
            EProcessInputContainer::TextDynamic(_) => container_category::TEXT_DYNAMIC,
            EProcessInputContainer::OutputFile(_) => container_category::OUTPUT_FILE,
            EProcessInputContainer::OutputLog(_) => container_category::OUTPUT_LOG,
            EProcessInputContainer::FrequencyPhantom => container_category::FREQUENCY_PHANTOM,
            EProcessInputContainer::OutputDevice(_) => container_category::OUTPUT_DEVICE,
            EProcessInputContainer::Dummy => container_category::DUMMY,
        }
    }

    /// 初期化済みか？
    pub fn is_initialized(&self) -> bool {
        self.as_container_category_flag() != container_category::UNINITIALIZED
    }

    /// `input_flag`からコンテナを初期化する。
    /// ただし既存状態が[`EProcessInputContainer::Uninitialized`]であること。
    pub fn initialize(&mut self, input_flag: EInputContainerCategoryFlag) {
        assert!(!self.is_initialized());

        *self = match input_flag {
            container_category::UNINITIALIZED | container_category::EMPTY => EProcessInputContainer::Empty,
            container_category::BUFFER_MONO_PHANTOM => EProcessInputContainer::BufferMonoPhantom,
            container_category::BUFFER_MONO_DYNAMIC => {
                EProcessInputContainer::BufferMonoDynamic(BufferMonoDynamicItem::new())
            }
            container_category::TEXT_DYNAMIC => EProcessInputContainer::TextDynamic(TextDynamicItem::new()),
            container_category::OUTPUT_LOG => {
                EProcessInputContainer::OutputLog(EOutputLogItem::TextDynamic(TextDynamicItem { buffer: vec![] }))
            }
            container_category::OUTPUT_FILE => {
                EProcessInputContainer::OutputFile(EOutputFileInput::Mono(BufferMonoDynamicItem::new()))
            }
            container_category::OUTPUT_DEVICE => {
                EProcessInputContainer::OutputDevice(EOutputDeviceInput::Mono(BufferMonoDynamicItem::new()))
            }
            container_category::FREQUENCY_PHANTOM => EProcessInputContainer::FrequencyPhantom,
            container_category::DUMMY => EProcessInputContainer::Dummy,
            _ => unreachable!("Unexpected branch"),
        }
    }

    /// `output`から中身を更新する。
    pub fn process(&mut self, output: &EProcessOutputContainer) {
        assert!(self.is_initialized());

        match self {
            EProcessInputContainer::Uninitialized | EProcessInputContainer::Empty => {
                // Emptyであるかをチェック。
                match output {
                    EProcessOutputContainer::Empty => {}
                    _ => unreachable!("Unexpected output"),
                }
            }
            EProcessInputContainer::BufferMonoPhantom => {
                // WaveBufferであるかをチェック。
                match output {
                    EProcessOutputContainer::BufferMono(_) => (),
                    _ => unreachable!("Unexpected output"),
                }
            }
            EProcessInputContainer::BufferMonoDynamic(dst) => {
                dst.update(output);
            }
            EProcessInputContainer::BufferStereoDynamic(dst) => {
                // WaveBufferであるかをチェック。
                match output {
                    // 記入する。
                    EProcessOutputContainer::BufferStereo(v) => {
                        dst.ch_left.append(&mut v.ch_left.clone());
                        dst.ch_right.append(&mut v.ch_right.clone());
                        dst.sample_rate = v.sample_rate;
                    }
                    _ => unreachable!("Unexpected output"),
                }
            }
            EProcessInputContainer::TextDynamic(dst) => match output {
                EProcessOutputContainer::Text(v) => {
                    dst.buffer.push(v.text.clone());
                }
                _ => unreachable!("Unexpected output"),
            },
            EProcessInputContainer::OutputFile(dst) => {
                // まずタイプが違うかをチェック。違ったら作りなおし。
                if !dst.can_support(output) {
                    dst.reset_with(output);
                }

                // そして入れる。ここからはタイプが同じであること前提。
                match dst {
                    EOutputFileInput::Mono(dst) => {
                        dst.update(output);
                    }
                    EOutputFileInput::Stereo(dst) => match output {
                        EProcessOutputContainer::BufferStereo(v) => {
                            dst.ch_left.append(&mut v.ch_left.clone());
                            dst.ch_right.append(&mut v.ch_right.clone());
                            dst.sample_rate = v.sample_rate;
                        }
                        _ => unreachable!("Unexpected output"),
                    },
                }
            }
            EProcessInputContainer::OutputLog(dst) => {
                // まずタイプが違うかをチェック。違ったら作りなおし。
                if !dst.can_support(output) {
                    dst.reset_with(output);
                }

                // そして入れる。ここからはタイプが同じであること前提。
                match dst {
                    EOutputLogItem::BuffersDynamic(dst) => {
                        // WaveBufferであるかをチェック。
                        match output {
                            // 記入する。
                            EProcessOutputContainer::BufferMono(v) => {
                                //dst.range = Some(v.range);
                                dst.sample_rate = v.sample_rate;
                                dst.buffer.append(&mut v.buffer.clone());
                            }
                            _ => unreachable!("Unexpected output"),
                        }
                    }
                    EOutputLogItem::TextDynamic(dst) => match output {
                        EProcessOutputContainer::Text(v) => {
                            dst.buffer.push(v.text.clone());
                        }
                        _ => unreachable!("Unexpected output"),
                    },
                }
            }
            EProcessInputContainer::OutputDevice(dst) => {
                // まずタイプが違うかをチェック。違ったら作りなおし。
                if !dst.can_support(output) {
                    dst.reset_with(output);
                }

                // そして入れる。ここからはタイプが同じであること前提。
                match dst {
                    EOutputDeviceInput::Mono(dst) => {
                        dst.update(output);
                    }
                    EOutputDeviceInput::Stereo(dst) => match output {
                        EProcessOutputContainer::BufferStereo(v) => {
                            dst.ch_left.append(&mut v.ch_left.clone());
                            dst.ch_right.append(&mut v.ch_right.clone());
                            dst.sample_rate = v.sample_rate;
                        }
                        _ => unreachable!("Unexpected output"),
                    },
                }
            }
            EProcessInputContainer::FrequencyPhantom => {}
            EProcessInputContainer::Dummy => {}
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
