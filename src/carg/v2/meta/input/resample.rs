use crate::resample::{ResampleHeaderSetting, ResampleSystem};

///
#[derive(Debug)]
pub(super) struct InputResampleController {
    ir_setting: ResampleHeaderSetting,
}

impl InputResampleController {
    pub fn new(from_fs: usize, to_fs: usize) -> Self {
        assert!(from_fs > 0);
        assert!(to_fs > 0);

        let proxy = ResampleSystem::get_proxy().unwrap();
        let setting = ResampleHeaderSetting {
            from_fs,
            to_fs,
            is_high_quality: true,
        };

        {
            let system = proxy.upgrade().unwrap();
            let mut system = system.lock().unwrap();
            system.create_response(&setting);
        }

        Self {
            ir_setting: setting,
        }
    }
}


// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
