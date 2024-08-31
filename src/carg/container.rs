use std::{
    collections::{HashMap, VecDeque},
    fs,
    io::{self, Write},
};

use itertools::Itertools;
use soundprog::wave::{
    analyze::window::EWindowFunction,
    container::WaveBuilder,
    sample::UniformedSample,
    setting::WaveSound,
    stretch::pitch::{PitchShifterBufferSetting, PitchShifterBuilder},
};

use super::{
    v1,
    v2::{self, Relation, RelationTreeNode},
};

/// @brief パーシングされたノードのコンテナ。
/// これだけで一連の処理ができる。
#[derive(Debug)]
pub enum ENodeContainer {
    None,
    V1 {
        input: Vec<v1::Input>,
        setting: v1::Setting,
        output: v1::Output,
    },
    V2 {
        setting: v2::Setting,
        nodes: HashMap<String, v2::ENode>,
        relations: Vec<v2::Relation>,
    },
}

impl ENodeContainer {
    pub fn process(&self) -> anyhow::Result<()> {
        match self {
            ENodeContainer::None => Ok(()),
            ENodeContainer::V1 { input, setting, output } => process_v1(input, setting, output),
            ENodeContainer::V2 {
                setting,
                nodes,
                relations,
            } => process_v2(setting, nodes, relations),
        }
    }
}

struct UniformedSampleBufferItem {
    buffer: Vec<UniformedSample>,
    start_index: usize,
    length: usize,
}

fn process_v1(input: &Vec<v1::Input>, setting: &v1::Setting, output: &v1::Output) -> anyhow::Result<()> {
    let fmt_setting = setting.as_wave_format_setting();

    let buffer = {
        let sample_rate = fmt_setting.samples_per_sec as u64;

        // Inputそれぞれをバッファー化する。
        let mut fallback_buffer_start_index = 0usize;
        let buffers = input
            .into_iter()
            .map(|v| {
                let sound_setting = v.into_sound_setting();
                let sound = WaveSound::from_setting(&fmt_setting, &sound_setting.sound);

                let mut buffer = vec![];
                for fragment in sound.sound_fragments {
                    buffer.extend(&fragment.buffer);
                }

                let buffer_length = buffer.len();
                let start_index = match sound_setting.explicit_buffer_start_index(sample_rate) {
                    Some(i) => i,
                    None => fallback_buffer_start_index,
                };
                let buffer_next_start_index = start_index + buffer_length;
                fallback_buffer_start_index = buffer_next_start_index.max(fallback_buffer_start_index);

                UniformedSampleBufferItem {
                    buffer,
                    start_index,
                    length: buffer_length,
                }
            })
            .collect_vec();

        // 合わせる。
        let buffer_length = buffers.iter().map(|v| v.start_index + v.length).max().unwrap();
        let mut new_buffer = vec![];
        new_buffer.resize_with(buffer_length, Default::default);

        for buffer in buffers {
            for src_i in 0..buffer.length {
                let dest_i = buffer.start_index + src_i;
                new_buffer[dest_i] += buffer.buffer[src_i];
            }
        }

        new_buffer
    };

    match output {
        v1::Output::File { format, file_name } => {
            let container = match format {
                v1::EOutputFileFormat::WavLPCM16 { sample_rate } => {
                    // もしsettingのsampling_rateがoutputのsampling_rateと違ったら、
                    // リサンプリングをしなきゃならない。
                    let source_sample_rate = setting.sample_rate as f64;
                    let dest_sample_rate = *sample_rate as f64;

                    let processed_container = {
                        let pitch_rate = source_sample_rate / dest_sample_rate;
                        if pitch_rate == 1.0 {
                            buffer
                        } else {
                            PitchShifterBuilder::default()
                                .pitch_rate(pitch_rate)
                                .window_size(128)
                                .window_function(EWindowFunction::None)
                                .build()
                                .unwrap()
                                .process_with_buffer(&PitchShifterBufferSetting { buffer: &buffer })
                                .unwrap()
                        }
                    };

                    WaveBuilder {
                        samples_per_sec: *sample_rate as u32,
                        bits_per_sample: match fmt_setting.bits_per_sample {
                            soundprog::wave::setting::EBitsPerSample::Bits16 => 16,
                        },
                    }
                    .build_container(processed_container)
                    .unwrap()
                }
            };

            // 書き込み。
            {
                let dest_file = fs::File::create(&file_name).expect("Could not create 500hz.wav.");
                let mut writer = io::BufWriter::new(dest_file);
                container.write(&mut writer);
                writer.flush().expect("Failed to flush writer.")
            }
        }
    }

    Ok(())
}

fn process_v2(setting: &v2::Setting, nodes: &HashMap<String, v2::ENode>, relations: &[Relation]) -> anyhow::Result<()> {
    v2::validate_node_relations(nodes, &relations)?;

    // チェックができたので(validation)、relationを元にGraphを生成する。
    // ただしそれぞれの独立したoutputをルートにして必要となるinputを子としてツリーを構成する。
    // outputがinputとして使われているものは一つの独立したツリーとして構成しない。
    let mut output_tree = vec![];
    let mut tree_node_map = HashMap::new();
    {
        // 各ノードから処理に使うためのアイテムを全部生成しておく。
        let mut process_datas = HashMap::new();
        for (node_name, node) in nodes {
            process_datas.insert(node_name, v2::ENodeProcessData::create_from(node, setting));
        }

        for (node_name, processor) in process_datas.into_iter() {
            let node = RelationTreeNode::new_item(&node_name, &processor);
            tree_node_map.insert(node_name.clone(), node);
        }
    }

    for node_name in v2::get_end_node_names(nodes, &relations).into_iter() {
        // root_nodeをqueueより寿命を長くする。
        let root_node = tree_node_map.get(&node_name).unwrap().clone();
        let mut pending_node_queue = VecDeque::new();
        pending_node_queue.push_back(root_node.clone());

        {
            while !pending_node_queue.is_empty() {
                let pending_node = pending_node_queue.pop_front().unwrap();

                for relation in relations {
                    let is_output = relation.output == *pending_node.borrow().name;
                    if !is_output {
                        continue;
                    }

                    // outputなrelationなので、inputを全部入れる。
                    let children = relation
                        .input
                        .iter()
                        .map(|name| tree_node_map.get(name).unwrap().clone())
                        .collect_vec();
                    pending_node.borrow_mut().append_children(children.clone());

                    // Queueにも入れる。
                    for child in children {
                        pending_node_queue.push_back(child);
                    }
                }
            }
        }

        // Outputのツリー
        output_tree.push(root_node);
    }
    if output_tree.is_empty() {
        return Ok(());
    }

    // そしてcontrol_itemsとnodes、output_treeを使って処理をする。+ setting.
    // VecDequeをStackのように扱って、DFSをコールスタックを使わずに実装することができそう。
    for output_root in &mut output_tree {
        let mut stack = vec![];
        stack.push(output_root.clone());

        while !stack.is_empty() {
            let result = stack.last_mut().unwrap().borrow_mut().try_process();

            match result {
                v2::EProcessResult::Finished => {
                    let _ = stack.pop().unwrap();
                }
                v2::EProcessResult::Pending => {
                    // reverseしてpop()するのがめんどくさいので、一旦これで。
                    let mut children = {
                        let item = stack.last_mut().unwrap();
                        item.borrow().try_get_unfinished_children_names()
                    };
                    if children.is_empty() {
                        println!("Children nodes must be exist when pending nodes.");
                        continue;
                    }

                    let next_node = tree_node_map.get(&children.pop().unwrap()).unwrap().clone();
                    stack.push(next_node);
                }
            }
        }
    }

    Ok(())
}
