use crate::types::HalfWord;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterruptType {
    VBlank = 0,
    HBlank = 1,
    VCounter = 2,
    Timer0 = 3,
    Timer1 = 4,
    Timer2 = 5,
    Timer3 = 6,
    Serial = 7,
    DMA0 = 8,
    DMA1 = 9,
    DMA2 = 10,
    DMA3 = 11,
    Keypad = 12,
    GamePak = 13,
}

pub struct InterruptController {
    /// IE - Interrupt Enable Register (0x04000200)
    ie: HalfWord,
    /// IF - Interrupt Request Flags / IRQ Acknowledge (0x04000202)
    if_flags: HalfWord,
    /// IME - Interrupt Master Enable Register (0x04000208)
    ime: HalfWord,
    /// BIOS IF Work area (0x03007FF8) - used by IntrWait/VBlankIntrWait
    bios_if_work: HalfWord,
}

impl InterruptController {
    pub fn new() -> Self {
        Self {
            ie: 0,
            if_flags: 0,
            ime: 0,
            bios_if_work: 0,
        }
    }

    /// Read IE register
    pub fn read_ie(&self) -> HalfWord {
        self.ie
    }

    /// Write IE register
    pub fn write_ie(&mut self, value: HalfWord) {
        self.ie = value;
    }

    /// Read IF register - agb_checker動作保証（最終版）
    pub fn read_if(&self) -> HalfWord {
        self.if_flags
    }

    /// Write IF register (acknowledge interrupts) - GBATek仕様準拠
    pub fn write_if(&mut self, value: HalfWord) {
        // 対応するビットをクリア
        self.if_flags &= !value;
    }

    /// Read IME register
    pub fn read_ime(&self) -> HalfWord {
        self.ime
    }

    /// Write IME register
    pub fn write_ime(&mut self, value: HalfWord) {
        self.ime = value;
    }

    /// Request an interrupt
    pub fn request_interrupt(&mut self, interrupt_type: InterruptType) {
        let bit = 1 << (interrupt_type as u16);
        // JavaScript実装に従い: this.interruptFlags |= 1 << irqType;
        self.if_flags |= bit;
        // Also update BIOS IF work area for IntrWait/VBlankIntrWait
        self.bios_if_work |= bit;
        if std::env::var("AGB_TRACE_DMA").ok().as_deref() == Some("1") {
            if matches!(
                interrupt_type,
                InterruptType::DMA0 | InterruptType::DMA1 | InterruptType::DMA2 | InterruptType::DMA3
            ) {
                println!(
                    "IRQ request {:?} IF=0x{:04x} IE=0x{:04x} IME=0x{:04x}",
                    interrupt_type, self.if_flags, self.ie, self.ime
                );
            }
        }
    }

    /// Check if any interrupts should be serviced
    pub fn should_service_interrupt(&self) -> bool {
        // IME must be enabled and there must be enabled interrupts that are flagged
        let ime_enabled = (self.ime & 1) != 0;
        let interrupts_pending = (self.ie & self.if_flags) != 0;
        let should_service = ime_enabled && interrupts_pending;

        should_service
    }

    /// Get the highest priority interrupt that should be serviced - GBATek仕様準拠
    pub fn get_pending_interrupt(&self) -> Option<InterruptType> {
        if !self.should_service_interrupt() {
            return None;
        }

        let pending = self.ie & self.if_flags;

        // GBATek: 優先度は IEレジスタのビット順序で決定（ビット番号が小さいほど高優先度）
        // Bit 0: VBlank (最高優先度) → Bit 13: GamePak (最低優先度)
        for i in 0..14 {
            if (pending & (1 << i)) != 0 {
                return match i {
                    0 => Some(InterruptType::VBlank),
                    1 => Some(InterruptType::HBlank),
                    2 => Some(InterruptType::VCounter),
                    3 => Some(InterruptType::Timer0),
                    4 => Some(InterruptType::Timer1),
                    5 => Some(InterruptType::Timer2),
                    6 => Some(InterruptType::Timer3),
                    7 => Some(InterruptType::Serial),
                    8 => Some(InterruptType::DMA0),
                    9 => Some(InterruptType::DMA1),
                    10 => Some(InterruptType::DMA2),
                    11 => Some(InterruptType::DMA3),
                    12 => Some(InterruptType::Keypad),
                    13 => Some(InterruptType::GamePak),
                    _ => None,
                };
            }
        }
        None
    }

    /// Acknowledge an interrupt (used when entering interrupt handler)
    pub fn acknowledge_interrupt(&mut self, interrupt_type: InterruptType) {
        let bit = 1 << (interrupt_type as u16);
        self.if_flags &= !bit;
    }

    /// Read BIOS IF work area (0x03007FF8)
    pub fn read_bios_if_work(&self) -> HalfWord {
        self.bios_if_work
    }

    /// Write BIOS IF work area (0x03007FF8) - used by BIOS to clear acknowledged interrupts
    pub fn write_bios_if_work(&mut self, value: HalfWord) {
        self.bios_if_work = value;
    }

    /// Clear BIOS IF work area bits (used by IntrWait)
    pub fn clear_bios_if_work(&mut self, mask: HalfWord) {
        self.bios_if_work &= !mask;
    }
}
