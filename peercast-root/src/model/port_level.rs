
/// ポートチェックされたPeerCastの疎通レベル
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PortLevel {
    /// ポートチェックしたが疎通できなかった
    Incomplete = -1,

    /// ポートチェックが行われていない
    None = 0,

    /// 疎通OK
    Welldone = 1,

    // 疎通OK, 配信速度OK
    WelldoneWithSpeed(u16) = 2,
}





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_restrect_level() {
        // assert_eq!(PortRestrictLevel::None, 0_u8);
        // assert_eq!(PortRestrictLevel::Welldone, 1_u8);
        // assert_eq!(PortRestrictLevel::WelldoneReachedUploadSpeed , 2_u8);
        // assert_eq!(PortRestrictLevel::WelldoneReachedRestrictSpeed(1000), 3_u8);
    }

    #[test]
    fn test_port_level() {
        assert!(PortLevel::Incomplete == PortLevel::Incomplete);
        assert!(PortLevel::Incomplete < PortLevel::None);
        assert!(PortLevel::Incomplete < PortLevel::Welldone);
        assert!(PortLevel::Incomplete < PortLevel::WelldoneWithSpeed(0));
        //
        assert!(PortLevel::None < PortLevel::Welldone);
        assert!(PortLevel::None < PortLevel::WelldoneWithSpeed(0));
        //
        assert!(PortLevel::Welldone < PortLevel::WelldoneWithSpeed(0));
        //
        assert!(PortLevel::WelldoneWithSpeed(0) < PortLevel::WelldoneWithSpeed(1));
        assert!(PortLevel::WelldoneWithSpeed(1) > PortLevel::WelldoneWithSpeed(0));
        assert!(PortLevel::WelldoneWithSpeed(1) == PortLevel::WelldoneWithSpeed(1));
        assert!(PortLevel::WelldoneWithSpeed(0) != PortLevel::WelldoneWithSpeed(1));
    }
}
