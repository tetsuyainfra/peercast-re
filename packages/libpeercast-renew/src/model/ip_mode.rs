#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum IpMode {
    IpV4 = 1_u32,
    IpV6 = 100_u32,
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn test_ip_mode() {
        assert_eq!(1_u32, IpMode::IpV4 as u32);
        assert_eq!(100_u32, IpMode::IpV6 as u32);
    }
}
