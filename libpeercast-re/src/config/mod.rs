use std::{
    fmt::Debug,
    fs::{self, File},
    io::Write,
    net::IpAddr,
    path::{Path, PathBuf},
    str::FromStr,
};

use ipnet::{IpNet, Ipv4Net};
use pbkdf2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Pbkdf2,
};
use rand_core::OsRng;
use tracing::{debug, info, warn};
use tracing_subscriber::field::debug;

use crate::{
    error::{AuthError, ConfigError, ParseVariableError},
    pcp::GnuId,
};

mod loader;
pub use loader::*;

const SECTION_SERVER: &str = "Server";
const SECTION_ROOT: &str = "Root";
const SECTION_PRIVACY: &str = "Privacy";

#[derive(Debug, Clone)]
pub struct Config {
    config_file_path: Option<PathBuf>,

    pub server_address: ConfigAddress,
    pub server_port: u16,
    pub rtmp_port: u16,
    pub local_address: Vec<IpNet>,
    pub root_mode: bool,
    pub root_session_id: Option<GnuId>,

    // Privacy
    pub username: Option<String>,
    pub password: Option<ConfigPassword>,
}

impl Config {
    pub fn load_str(s: &str) -> Result<Self, ConfigError> {
        let conf = ini::Ini::load_from_str(&s)?;

        let Config {
            config_file_path,
            // Server
            server_address,
            server_port,
            rtmp_port,
            local_address,
            // Privacy
            username,
            password,
            // Root
            root_mode,
            root_session_id,
        } = Config::default();

        let (server_address, server_port, rtmp_port, local_address) = match conf
            .section(Some(SECTION_SERVER))
        {
            None => (server_address, server_port, rtmp_port, local_address),
            Some(sec) => {
                let server_address = match sec.get("server_address") {
                    None | Some("") => server_address,
                    Some(s) => {
                        let ip = s
                            .parse::<IpAddr>()
                            .map_err(|e| ParseVariableError::from(e))?;
                        ConfigAddress::Config(ip)
                    }
                };
                let server_port = match sec.get("server_port") {
                    None | Some("") => server_port,
                    Some(s) => s.parse::<u16>().map_err(|e| ParseVariableError::from(e))?,
                };
                let rtmp_port = match sec.get("rtmp_port") {
                    None | Some("") => rtmp_port,
                    Some(s) => s.parse::<u16>().map_err(|e| ParseVariableError::from(e))?,
                };
                let local_address = match sec.get("local_address") {
                    None | Some("") => local_address,
                    Some(s) => serde_json::from_str(s).map_err(|e| ParseVariableError::from(e))?,
                };
                (server_address, server_port, rtmp_port, local_address)
            }
        };

        let (username, password) = match conf.section(Some(SECTION_PRIVACY)) {
            None => (username, password),
            Some(sec) => {
                let username = match sec.get("username") {
                    None | Some("") => username,
                    Some(s) => Some(s.to_string()),
                };
                let password = match sec.get("password") {
                    None | Some("") => password,
                    Some(s) => {
                        match PasswordHash::new(s) {
                            // Hash化されたパスワード
                            Ok(_hashed) => {
                                info!("{SECTION_PRIVACY}.password read.");
                                Some(ConfigPassword::Hashed(s.to_string()))
                            }
                            // 平文のパスワード
                            Err(e) => {
                                warn!("config password is error occured({e}). if you set Plain Text to {SECTION_PRIVACY}.password, don't reminde this message.");
                                Some(ConfigPassword::Plain(s.to_string()))
                            }
                        }
                    }
                };
                (username, password)
            }
        };

        let (root_mode, root_session_id) = match conf.section(Some(SECTION_ROOT)) {
            None => (root_mode, root_session_id),
            Some(sec) => {
                //
                let root_mode = match sec.get("root_mode") {
                    None | Some("") => root_mode,
                    Some(s) => s.parse::<bool>().map_err(|e| ParseVariableError::from(e))?,
                };
                let root_session_id = match sec.get("root_session_id") {
                    None | Some("") => root_session_id,
                    Some(s) => Some(GnuId::from_str(s).map_err(|e| ParseVariableError::from(e))?),
                };

                (root_mode, root_session_id)
            }
        };

        Ok(Config {
            config_file_path,
            server_address,
            server_port,
            rtmp_port,
            local_address,
            // Privacy
            username,
            password,
            // Root
            root_mode,
            root_session_id,
        })
    }

    // pub fn save_str(&self) -> Result<Vec<u8>, ConfigError> {
    pub fn save_str(&self) -> Vec<u8> {
        let mut ini = ini::Ini::new();
        ini.with_section(Some(SECTION_SERVER))
            .set("server_address", &self.server_address)
            .set("server_port", &self.server_port.to_string())
            .set("rtmp_port", &self.rtmp_port.to_string())
            .set(
                "permit_address",
                serde_json::to_string(&self.local_address).unwrap(),
            );

        ini.with_section(Some(SECTION_PRIVACY))
            .set(
                "username",
                self.username
                    .as_ref()
                    .map_or(String::new(), |name| name.clone()),
            )
            .set(
                "password",
                self.password.as_ref().map_or(String::new(), |pw| pw.into()),
            );
        ini.with_section(Some(SECTION_ROOT))
            .set("root_mode", &self.root_mode.to_string())
            .set(
                "root_session_id",
                &self
                    .root_session_id
                    .as_ref()
                    .map_or(String::new(), |id| id.to_string()),
            );

        let mut buf = Vec::new();
        let _r = ini.write_to(&mut buf).unwrap();
        buf
    }
}

impl ConfigTrait for Config {
    type ErrorType = ConfigError;
    fn load_file(path: &PathBuf) -> Result<Self, Self::ErrorType> {
        let file_str = fs::read_to_string(path)?;
        let mut config = Self::load_str(&file_str)?;

        config.config_file_path = Some(PathBuf::from(path));
        Ok(config)
    }

    fn save_file(&self, path: &PathBuf) -> Result<(), Self::ErrorType> {
        let buf = self.save_str();
        let mut file = File::create(path)?;
        let _r = file.write_all(&buf)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_file_path: None,
            server_address: ConfigAddress::NoConfig("0.0.0.0".parse().unwrap()),
            server_port: 17144,
            rtmp_port: 11935,
            local_address: vec!["127.0.0.0/8".parse().unwrap()],
            root_mode: false,
            root_session_id: None,
            //
            username: None,
            password: None,
        }
    }
}

impl ToString for Config {
    fn to_string(&self) -> String {
        let v = self.save_str();
        String::from_utf8(v).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigAddress {
    NoConfig(IpAddr),
    Config(IpAddr),
}

impl ConfigAddress {
    pub fn to_ipaddr(&self) -> IpAddr {
        match self {
            ConfigAddress::NoConfig(s) => s.clone(),
            ConfigAddress::Config(s) => s.clone(),
        }
    }
}

impl From<&ConfigAddress> for String {
    fn from(value: &ConfigAddress) -> Self {
        match value {
            ConfigAddress::NoConfig(_v) => String::new(),
            ConfigAddress::Config(v) => v.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigPassword {
    Plain(String),
    Hashed(String),
}

impl ConfigPassword {
    #[allow(dead_code)]
    fn verify_password(&self, password: &str) -> Result<(), AuthError> {
        match self {
            ConfigPassword::Plain(plain) => {
                if plain == password {
                    Ok(())
                } else {
                    Err(AuthError::WrongPassword)
                }
            }
            ConfigPassword::Hashed(hashed_password) => {
                let parsed_hash = PasswordHash::new(&hashed_password).unwrap();
                match Pbkdf2.verify_password(password.as_bytes(), &parsed_hash) {
                    Ok(_) => Ok(()),
                    Err(_e) => Err(AuthError::WrongPassword),
                }
            }
        }
    }

    fn to_hashed(&self) -> Self {
        match self {
            ConfigPassword::Hashed(hashed) => ConfigPassword::Hashed(hashed.clone()),
            ConfigPassword::Plain(plain_password) => {
                let salt = SaltString::generate(&mut OsRng);
                // Hash password to PHC string ($pbkdf2-sha256$...)
                let hashed_password = Pbkdf2
                    .hash_password(plain_password.as_bytes(), &salt)
                    .unwrap()
                    .to_string();
                ConfigPassword::Hashed(hashed_password)
            }
        }
    }
}

impl From<&ConfigPassword> for String {
    fn from(value: &ConfigPassword) -> Self {
        match value.to_hashed() {
            ConfigPassword::Plain(_p) => panic!("password is plain text"),
            ConfigPassword::Hashed(h) => h,
        }
    }
}
// 無くてもよい気がする
// impl From<ConfigPassword> for String {
//     fn from(value: ConfigPassword) -> Self {
//         (&value).into()
//     }
// }

#[cfg(test)]
mod test {
    use std::{net::Ipv4Addr, path::PathBuf, process::exit};

    use minijinja::render;

    use crate::error::ParseVariableError;

    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(
            config.server_address,
            ConfigAddress::NoConfig(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)))
        );
        assert_eq!(config.server_port, 17144_u16);
        assert_eq!(config.username, None);
        assert_eq!(config.password, None);
        assert_eq!(config.local_address, vec!["127.0.0.0/8".parse().unwrap()]);
    }

    /// config.example.iniへのパス
    fn path_to_example_ini() -> PathBuf {
        let example_file = Path::new(file!());
        let mut dir_buf = PathBuf::from(example_file.parent().unwrap());

        let path_buf = dir_buf.join("config.example.ini");
        path_buf
    }

    #[test]
    fn test_config_loading() {
        // 空のファイルだった時の処理
        let conf = Config::load_str("").unwrap();
        assert_eq!(conf.config_file_path, None);

        let s = render!(include_str!("config.test.ini.j2"));
        let conf = Config::load_str(&s).unwrap();
        assert_eq!(conf.config_file_path, None);

        let config = Config::load_file(&path_to_example_ini()).unwrap();
        assert_eq!(&config.config_file_path.unwrap(), &path_to_example_ini());
    }

    #[ignore = "exec by user only"]
    #[test]
    fn create_config() {
        let config = Config::default();
        let _ret = config.save_file(&path_to_example_ini()).unwrap();
        let _ret = Config::load_file(&path_to_example_ini()).unwrap();
    }

    #[test]
    fn config_tests() {
        let s = render!(include_str!("config.test.ini.j2"),  server_port => 1);
        let conf = Config::load_str(&s).unwrap();
        let def_conf = Config::default();
        assert_eq!(conf.server_address, def_conf.server_address);
        assert_eq!(conf.server_port, 1);
        assert_eq!(conf.rtmp_port, def_conf.rtmp_port);
        assert_eq!(conf.local_address, def_conf.local_address);
        assert_eq!(conf.username, def_conf.username);
        assert_eq!(conf.password, def_conf.password);
        assert_eq!(conf.root_mode, def_conf.root_mode);

        let s = render!(include_str!("config.test.ini.j2"),  server_port => 1, password=>"plain_password");
        let conf = Config::load_str(&s).unwrap();
        assert_eq!(
            conf.password,
            Some(ConfigPassword::Plain("plain_password".to_string()))
        );
    }

    #[test]
    fn create_config_errors() -> () {
        let s = render!(include_str!("config.test.ini.j2"),  server_port => 1);
        let _conf = Config::load_str(&s).unwrap();

        // server_port こいつは必ず整数なのでエラー分かりやすい
        let s = render!(include_str!("config.test.ini.j2"),  server_port => ".1",);
        match Config::load_str(&s) {
            Err(ConfigError::ParseVariable(e)) => match e {
                ParseVariableError::Integer(_) => assert!(true),
                _ => assert!(false),
            },
            _ => assert!(false),
        };

        // server_address error
        let s = render!(include_str!("config.test.ini.j2"),  server_address => "1");
        match Config::load_str(&s) {
            Err(ConfigError::ParseVariable(e)) => match e {
                ParseVariableError::Ip(_) => assert!(true),
                _ => assert!(false),
            },
            _ => assert!(false),
        };

        // local_address
        let s = render!(include_str!("config.test.ini.j2"),  local_address => "127.0.0.1"); // 単発にするとエラー
        match Config::load_str(&s) {
            Err(ConfigError::ParseVariable(e)) => match e {
                ParseVariableError::Serde(_) => assert!(true),
                _ => assert!(false),
            },
            _ => assert!(false),
        };
        let s = render!(include_str!("config.test.ini.j2"),  local_address => ["127.0.0.1"]); // X.X.X.X/Subnet を忘れるとエラー
        match Config::load_str(&s) {
            Err(ConfigError::ParseVariable(ParseVariableError::Serde(_))) => assert!(true),
            _ => assert!(false),
        };

        let s = render!(include_str!("config.test.ini.j2"),  local_address => ["127.0.0.1/24"]); // Okパターン
        match Config::load_str(&s) {
            Ok(_) => assert!(true),
            _ => assert!(false),
        }

        // root_mode
        let s = render!(include_str!("config.test.ini.j2"),  root_mode => 1);
        match Config::load_str(&s) {
            Err(ConfigError::ParseVariable(e)) => match e {
                ParseVariableError::Bool(_) => assert!(true),
                _ => assert!(false),
            },
            _ => assert!(false),
        };

        // root_session_id
        let s = render!(include_str!("config.test.ini.j2"),  root_session_id => 1);
        match Config::load_str(&s) {
            Err(ConfigError::ParseVariable(e)) => match e {
                ParseVariableError::GnuId(_) => assert!(true),
                _ => assert!(false),
            },
            _ => assert!(false),
        };

        // username
        // MEMO: 今のところエラーになる表現無し(usernameにはどんな文字でも使える)

        // password
        // MEMO: 今のところエラーになる表現無し(passwordにはどんな文字でも使える | エラーになる文字列はplain textとして扱っている)
    }

    #[ignore = "hashingが重いので無効化"]
    #[test]
    fn test_config_password() {
        let p = ConfigPassword::Plain("7144".to_string());
        assert!(p.verify_password("7144").is_ok());
        assert!(p.verify_password("7145").is_err());
        let p = ConfigPassword::Plain("7144".to_string()).to_hashed();
        assert!(p.verify_password("7144").is_ok());
        assert!(p.verify_password("7145").is_err());
    }

    #[ignore = "spec test"]
    #[test]
    fn path_buf() {
        let p = PathBuf::from("~/");
        println!("{p:?}");
    }

    #[ignore = "spec test"]
    #[test]
    fn ipnet_testing() {
        use ipnet::{IpNet, Ipv4Net, Ipv6Net};
        use std::net::{Ipv4Addr, Ipv6Addr};
        use std::str::FromStr;

        let net4 = Ipv4Net::new(Ipv4Addr::new(10, 1, 1, 0), 24).unwrap();
        let net6 = Ipv6Net::new(Ipv6Addr::new(0xfd, 0, 0, 0, 0, 0, 0, 0), 24).unwrap();
        let mut nets = Vec::new();
        nets.push(IpNet::from(net4));
        nets.push(IpNet::from(net6));

        let s = serde_json::to_string(&nets).unwrap();
        println!("{}", s);

        // let mut ini = ini::Ini::new(); ini.with_section(Some(super::SECTION_SERVER)).set("nets", c);
    }
}
