use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::{EmitterRange, Setting};
use crate::wave::sample::UniformedSample;

/// [`EProcessInputContainer`]の各アイテムの識別子をまとめている。
pub mod container_category {
    pub const UNINITIALIZED: u64 = 0;

    /// 空
    pub const EMPTY: u64 = 1 << 0;

    /// 動的にWaveBufferを保持するコンテナとして運用する。
    pub const WAVE_BUFFERS_DYNAMIC: u64 = 1 << 1;

    /// バッファを受け取るにはするが、内部処理はしない。
    pub const WAVE_BUFFER_PHANTOM: u64 = 1 << 2;

    /// 動的にTextのリストを保持するコンテナとして運用する。
    pub const TEXT_DYNAMIC: u64 = 1 << 3;

    /// [`ENodeSpecifier::OutputLog`]専用
    pub const OUTPUT_LOG: u64 = WAVE_BUFFERS_DYNAMIC | TEXT_DYNAMIC;
}

pub type EInputContainerCategoryFlag = u64;

#[derive(Debug, Clone)]
pub enum EProcessInputContainer {
    Uninitialized,
    Empty,
    WaveBuffersDynamic(WaveBufferDynamicItem),
    WaveBufferPhantom,
    TextDynamic(TextDynamicItem),
    OutputLog(EOutputLogItem),
}

/// [`EProcessInputContainer::WaveBuffersDynamic`]の内部コンテナ
#[derive(Debug, Clone)]
pub struct WaveBufferDynamicItem {
    pub buffer: Vec<UniformedSample>,
    pub range: Option<EmitterRange>,
    pub setting: Option<Setting>,
}

impl WaveBufferDynamicItem {
    fn new() -> Self {
        Self {
            buffer: vec![],
            range: None,
            setting: None,
        }
    }
}

/// [`EProcessInputContainer::TextDynamic`]の内部コンテナ
#[derive(Debug, Clone)]
pub struct TextDynamicItem {
    buffer: Vec<String>,
}

impl TextDynamicItem {
    fn new() -> Self {
        Self { buffer: vec![] }
    }
}

/// [`EProcessInputContainer::OutputLog`]の内部コンテナ
#[derive(Debug, Clone)]
pub enum EOutputLogItem {
    BuffersDynamic(WaveBufferDynamicItem),
    TextDynamic(TextDynamicItem),
}

impl EOutputLogItem {
    /// 今のセッティングで`output`が受け取れるか？
    fn can_support(&self, output: &EProcessOutputContainer) -> bool {
        match self {
            EOutputLogItem::BuffersDynamic(_) => {
                match output {
                    EProcessOutputContainer::Empty |
                    EProcessOutputContainer::WaveBuffer(_) =>  true,
                    _ => false,
                }
            }
            EOutputLogItem::TextDynamic(_) => {
                match output {
                    EProcessOutputContainer::Empty |
                    EProcessOutputContainer::Text(_) => true,
                    _ => false,
                }
            }
        }
    }

    /// `output`からセッティングをリセットする。
    fn reset_with(&mut self, output: &EProcessOutputContainer) {
        if self.can_support(output) {
            return;
        }

        match output {
            EProcessOutputContainer::Empty => unreachable!("Unexpected branch"),
            EProcessOutputContainer::WaveBuffer(_) => {
                *self = Self::BuffersDynamic(WaveBufferDynamicItem::new());
            }
            EProcessOutputContainer::Text(_) => {
                *self = Self::TextDynamic(TextDynamicItem::new());
            }
            EProcessOutputContainer::Frequency(_) => unimplemented!(),
        }
    }

    /// 種類をかえずに中身だけをリセットする。
    pub fn reset(&mut self) {
        match self {
            EOutputLogItem::BuffersDynamic(v) => {
                *v = WaveBufferDynamicItem::new();
            }
            EOutputLogItem::TextDynamic(v) => {
                v.buffer.clear();
            }
        }
    }
}

impl EProcessInputContainer {
    /// 現在保持しているInputコンテナを識別するためのフラグを返す。
    pub fn as_container_category_flag(&self) -> EInputContainerCategoryFlag {
        match self {
            EProcessInputContainer::Uninitialized => container_category::UNINITIALIZED,
            EProcessInputContainer::WaveBufferPhantom => container_category::WAVE_BUFFER_PHANTOM,
            EProcessInputContainer::Empty => container_category::EMPTY,
            EProcessInputContainer::WaveBuffersDynamic(_) => container_category::WAVE_BUFFERS_DYNAMIC,
            EProcessInputContainer::TextDynamic(_) => container_category::TEXT_DYNAMIC,
            EProcessInputContainer::OutputLog(_) => container_category::OUTPUT_LOG,
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
            container_category::WAVE_BUFFER_PHANTOM => EProcessInputContainer::WaveBufferPhantom,
            container_category::WAVE_BUFFERS_DYNAMIC => {
                EProcessInputContainer::WaveBuffersDynamic(WaveBufferDynamicItem::new())
            }
            container_category::TEXT_DYNAMIC => EProcessInputContainer::TextDynamic(TextDynamicItem::new()),
            container_category::OUTPUT_LOG => {
                EProcessInputContainer::OutputLog(EOutputLogItem::TextDynamic(TextDynamicItem { buffer: vec![] }))
            }
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
            EProcessInputContainer::WaveBufferPhantom => {
                // WaveBufferであるかをチェック。
                match output {
                    EProcessOutputContainer::WaveBuffer(_) => (),
                    _ => unreachable!("Unexpected output"),
                }
            }
            EProcessInputContainer::WaveBuffersDynamic(dst) => {
                // WaveBufferであるかをチェック。
                match output {
                    // 記入する。
                    EProcessOutputContainer::WaveBuffer(v) => {
                        dst.range = Some(v.range);
                        dst.setting = Some(v.setting.clone());
                        dst.buffer.append(&mut v.buffer.clone());
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
                            EProcessOutputContainer::WaveBuffer(v) => {
                                dst.range = Some(v.range);
                                dst.setting = Some(v.setting.clone());
                                dst.buffer.append(&mut v.buffer.clone());
                            }
                            _ => unreachable!("Unexpected output"),
                        }
                    }
                    EOutputLogItem::TextDynamic(dst) => {
                        match output {
                            EProcessOutputContainer::Text(v) => {
                                dst.buffer.push(v.text.clone());
                            }
                            _ => unreachable!("Unexpected output"),
                        }
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
