use super::*;
use crate::types::*;

bitfield! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct DISPCNT(u16);
    pub obj_display_flag, _: 15;
    pub window1_display_flag, _: 14;
    pub window0_display_flag, _: 13;
    pub screen_display_obj, _: 12;
    pub screen_display_bg3, _: 11;
    pub screen_display_bg2, _: 10;
    pub screen_display_bg1, _: 9;
    pub screen_display_bg0, _: 8;
    pub forced_vlank, _: 7;
    pub obj_char_mapping, _: 6;
    pub hblank_interbal_free, _: 5;
    pub display_frame_select, _: 4;
    pub bg_mode, _: 2, 0;
}

impl DISPCNT {
    pub fn new() -> Self {
        DISPCNT::default()
    }

    pub fn write(&mut self, data: HalfWord) {
        self.0 = data
    }
}

impl Default for DISPCNT {
    fn default() -> Self {
        DISPCNT(0x0080)
    }
}
