use lv2hm::Lv2Host;

pub(crate) mod delay;


pub(crate) struct LV2Wrapper(Lv2Host);

unsafe impl Send for LV2Wrapper {}

impl LV2Wrapper {
    pub fn new(plugin_cap: usize, buffer_len: usize, sample_rate: usize) -> Self {
        Self(Lv2Host::new(plugin_cap, buffer_len, sample_rate))
    }
}

impl std::ops::Deref for LV2Wrapper {
    type Target = Lv2Host;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for LV2Wrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}