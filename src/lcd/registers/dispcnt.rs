use super::*;
use crate::types::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BgMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
    Mode4,
    Mode5,
}

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

    pub fn mode(&self) -> BgMode {
        match self.bg_mode() {
            0x00 => BgMode::Mode0,
            0x01 => BgMode::Mode1,
            0x02 => BgMode::Mode2,
            0x03 => BgMode::Mode3,
            0x04 => BgMode::Mode4,
            0x05 => BgMode::Mode5,
            _ => unreachable!("bg mode should be 0~5."),
        }
    }

    pub fn write(&mut self, data: HalfWord) {
        self.0 = data
    }

    pub fn read(&self) -> HalfWord {
        self.0
    }
}

impl Default for DISPCNT {
    fn default() -> Self {
        DISPCNT(0x0080)
    }
}
