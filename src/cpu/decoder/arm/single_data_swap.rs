bitfield! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct SingleDataSwap(u32);
    pub get_cond, _: 31, 28;
    #[allow(non_snake_case)]
    pub get_B, _: 22;
    #[allow(non_snake_case)]
    pub get_Rn, _: 19, 16;
    #[allow(non_snake_case)]
    pub get_Rd, _: 15, 12;
    #[allow(non_snake_case)]
    pub get_Rm, _: 3, 0;
}
