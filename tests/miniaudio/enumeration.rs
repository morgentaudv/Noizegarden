
use miniaudio::{Context, DeviceId, DeviceType, ShareMode};

/// -- --nocaptureをつけてテストすること。
#[test]
fn test_miniaudio_enumerate_devices() {
    let context = Context::new(&[], None).expect("Failed to create context");

    context.with_devices(|playback_devices, capture_devices| {
        println!("Playback Devices");
        for (idx, device) in playback_devices.iter().enumerate() {
            println!("\tDevice #{}: {}", idx, device.name());
            print_device_info(&context, DeviceType::Playback, device.id());
        }

        println!("Capture Devices");
        for (idx, device) in capture_devices.iter().enumerate() {
            println!("\tDevice #{}: {}", idx, device.name());
            print_device_info(&context, DeviceType::Capture, device.id());
        }
    })
        .expect("Failed to get devices");
}

fn print_device_info(context: &Context, device_type: DeviceType, device_id: &DeviceId) {
    let info = match context.get_device_info(device_type, device_id, ShareMode::Shared) {
        Ok(info) => info,
        Err(err) => {
            eprintln!("\t\tFailed to get device info: {}", err);
            return;
        }
    };

    println!("\t\tSample Rate: {}-{}Hz", info.min_sample_rate(), info.max_sample_rate());
    println!("\t\tChannels: {}-{}", info.min_channels(), info.max_channels());
    println!("\t\tFormats: {:?}", info.formats());
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
