use crate::types::*;

bitfield! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct BGCNT(u16);
    pub screen_size, _: 15, 14; // Scree Size
    bg2_3_display_area_overflow, _: 13; // 0=Transparent 1=Wraparound
    pub screen_base_block, _: 12, 8; // 0-31 in units of 2KB, BG map data
    pub colors_palettes, _: 7; // 0=16/16, 1=256/1
    pub mosaic, _: 6;
    pub character_base_block, _: 3, 2; // bg tile
    pub bg_priority, _: 1, 0; // 0 = highest
}

impl BGCNT {
    pub fn new() -> Self {
        BGCNT::default()
    }

    pub fn bg_tile_offset(&self) -> Word {
        self.character_base_block() as Word * 0x4000
    }

    pub fn bg_map_offset(&self) -> Word {
        self.screen_base_block() as Word * 0x800
    }

    pub fn write(&mut self, data: HalfWord) {
        self.0 = data
    }

    pub fn read(&self) -> HalfWord {
        self.0
    }
}

impl Default for BGCNT {
    fn default() -> Self {
        BGCNT(0x0000)
    }
}
