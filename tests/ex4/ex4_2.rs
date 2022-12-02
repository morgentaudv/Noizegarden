use std::{fs, io};

use soundprog::wave::{
    analyze::{EAnalyzeMethod, EWindowFunction, FrequencyAnalyzerBuilder},
    container::WaveContainer,
};

#[test]
fn test_dft_hann() {
    const FILE_PATH: &'static str = "assets/ex4/sine_500hz.wav";

    let wave_container = {
        let source_file = fs::File::open(FILE_PATH).expect("Could not find a.wav.");
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };

    // DFT
    //{
    //    let analyzer = FrequencyAnalyzerBuilder::default()
    //        .time_start(0.0)
    //        .time_length(1.0)
    //        .time_precision(1.0 / 44100f64)
    //        .frequency_start(0f32)
    //        .frequency_length(2000f32)
    //        .frequency_precision(1f32)
    //        .window_function(Some(EWindowFunction::Hann))
    //        .build()
    //        .unwrap();

    //    let frequencies = analyzer.analyze_frequencies(&wave_container).unwrap();
    //    println!("DFT");
    //    for frequency in frequencies {
    //        println!("{:?}", frequency);
    //    }
    //}

    // FFT
    {
        let analyzer = FrequencyAnalyzerBuilder::default()
            .time_start(0.0)
            .time_length(1.0)
            .time_precision(1.0 / 64f64)
            .frequency_start(0f32)
            .frequency_length(6400f32)
            .frequency_precision(128f32)
            .window_function(Some(EWindowFunction::Hann))
            .analyze_method(EAnalyzeMethod::FFT)
            .build()
            .unwrap();

        let frequencies = analyzer.analyze_frequencies(&wave_container).unwrap();
        println!("FFT");
        for frequency in frequencies {
            println!("{:?}", frequency);
        }
    }
}
