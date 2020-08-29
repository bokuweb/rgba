bitfield! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct Memory(u32);
    pub get_cond, _: 31, 28;
    #[allow(non_snake_case)]
    pub get_I, _: 25;
    #[allow(non_snake_case)]
    pub get_P, _: 24;
    #[allow(non_snake_case)]
    pub get_U, _: 23;
    #[allow(non_snake_case)]
    pub get_B, _: 22;
    #[allow(non_snake_case)]
    pub get_W, _: 21;
    #[allow(non_snake_case)]
    pub get_L, _: 20;
    #[allow(non_snake_case)]
    pub get_Rn, _: 19, 16;
    #[allow(non_snake_case)]
    pub get_Rd, _: 15, 12;
    #[allow(non_snake_case)]
    pub get_imm, _: 11, 0;
    pub get_shamt5, _: 11, 7;
    pub get_sh, _: 6, 5;
    #[allow(non_snake_case)]
    pub get_bit4, _: 4;
    #[allow(non_snake_case)]
    pub get_Rm, _: 3, 0;
}
