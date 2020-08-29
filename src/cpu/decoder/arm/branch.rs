bitfield! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct Branch(u32);
    pub get_cond, _: 31, 28;
    #[allow(non_snake_case)]
    pub get_L, _: 24;
    pub get_offset, _: 23, 0;
}
