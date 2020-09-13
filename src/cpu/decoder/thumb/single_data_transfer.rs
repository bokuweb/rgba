use crate::cpu::types::HalfWord;

use super::*;

bitfield! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct SingleDataTransfer(u16);
    #[allow(non_snake_case)]
    pub get_B, _: 12;
    #[allow(non_snake_case)]
    pub get_L, _: 11;
    pub get_op, _: 11, 9;
    #[allow(non_snake_case)]
    pub get_Rd10_8, _: 10, 8;
    pub get_off5, _: 10, 6;
    #[allow(non_snake_case)]
    pub get_Rm, _: 8, 6;
    pub get_off8, _: 7, 0;
    #[allow(non_snake_case)]
    pub get_Rn, _: 5, 3;
    #[allow(non_snake_case)]
    pub get_Rd2_0, _: 2, 0;
}

