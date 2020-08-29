bitfield! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct BranchAndExchange(u32);
    pub get_cond, _: 31, 28;
    #[allow(non_snake_case)]
    pub get_L, _: 5;
    #[allow(non_snake_case)]
    pub get_Rm, _: 3, 0;
}
