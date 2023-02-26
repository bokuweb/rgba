bitfield! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct Branch(u16);
    #[allow(non_snake_case)]
    pub get_H11, _: 11;
    pub get_cond, _: 11, 8;
    pub get_offset11, _: 10, 0;
    pub get_offset10, _: 10, 1;
    pub get_offset8, _: 7, 0;
    #[allow(non_snake_case)]
    pub get_L, _: 7;
    pub get_bit7, _: 7;
    #[allow(non_snake_case)]
    pub get_H6, _: 6;
    #[allow(non_snake_case)]
    pub get_Rm, _: 6, 3;
}
