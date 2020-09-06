use crate::cpu::types::HalfWord;

use super::*;

bitfield! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct BlockDataTransfer(u16);
    #[allow(non_snake_case)]
    pub get_L, _: 11;
    #[allow(non_snake_case)]
    pub get_Rn, _: 10, 8;
    #[allow(non_snake_case)]
    pub get_R, _: 8;
    pub get_register_list, _: 7, 0;
}
