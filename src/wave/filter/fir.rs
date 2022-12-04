use crate::wave::{
    container::WaveContainer,
    filter::{compute_fir_lpf_filters_count, compute_fir_lpf_response},
    sample::UniformedSample,
};

pub(super) struct FIRLowPassInternal {
    /// エッジ周波数
    pub(super) edge_frequency: f64,
    /// 遷移帯域幅の総周波数範囲
    pub(super) delta_frequency: f64,
}

impl FIRLowPassInternal {
    pub(super) fn apply(&self, container: &WaveContainer) -> WaveContainer {
        // ここではcontainerのチャンネルがMONO(1)だと仮定する。
        assert!(container.channel() == 1);

        // まずLPFでは標本周波数が1として前提して計算を行うので、edgeとdeltaも変換する。
        let samples_per_sec = container.samples_per_second() as f64;
        let edge = self.edge_frequency / samples_per_sec;
        let delta = self.delta_frequency / samples_per_sec;

        // フィルタ係数の数を計算する。
        // フィルタ係数の数は整数になるしかないし、またfilters_count+1が奇数じゃなきゃならない。
        // (Window Functionをちゃんと決めるため)
        let filters_count = compute_fir_lpf_filters_count(delta);
        let filter_responses = compute_fir_lpf_response(filters_count, edge);

        // filter_responsesを用いて折りたたみを行う。
        let mut new_buffer = vec![];
        let orig_container = container.uniformed_sample_buffer();
        new_buffer.resize(orig_container.len(), UniformedSample::default());
        for sample_i in 0..new_buffer.len() {
            for fc_i in 0..=filters_count {
                if sample_i < fc_i {
                    break;
                }

                new_buffer[sample_i] += filter_responses[fc_i] * orig_container[sample_i - fc_i];
            }
        }

        WaveContainer::from_uniformed_sample_buffer(container, new_buffer)
    }
}
