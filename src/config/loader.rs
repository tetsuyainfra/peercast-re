use std::{
    fs::{self, File},
    marker::PhantomData,
    path::{Path, PathBuf},
};

use tracing::info;

//
pub trait ConfigTrait
where
    Self: Sized,
{
    type ErrorType: std::fmt::Debug;
    fn load_file(path: &Path) -> Result<Self, Self::ErrorType>;
    fn save_file(&self, path: &Path) -> Result<(), Self::ErrorType>;
}

// ConfigPathはConfigLoaderでPathを指定するときに使う変数
#[derive(Debug)]
pub enum ConfigPath {
    Env(String),
    Path(String),
    PathBuf(PathBuf),
}

impl ConfigPath {
    fn to_path(&self) -> Option<PathBuf> {
        match self {
            ConfigPath::Env(s) => match std::env::var(s) {
                Ok(s) => Some(PathBuf::from(s)),
                Err(_s) => None,
            },
            ConfigPath::Path(s) => Some(PathBuf::from(s)),
            ConfigPath::PathBuf(b) => Some(b.clone()),
        }
    }
}

/// ConfigLoaderは指定された設定ファイルを順次読み込み、指定された構造体Tを返すローダーである。
/// ローダーは指定されたパスを順番に読み込む。読み込んだファイルがエラーになった場合、そこでエラーを返す。
/// パスが指定されていない場合、panicが起きる
/// 構造体TはConfigTraitとDefaultトレイトを定義する必要がある。
/// example:
/// let c : Config = ConfigLoader::new()
///                     .add_source(ConfigPath::Env("HOGE_PATH"))
///                     .add_source(ConfigPath::Path("/HOGE/peercast-rt/peercast-rt.ini"))     // 実行ファイルのパス等
///                     .add_source(ConfigPath::Path("~/.peercast-rt.ini"))                    // デフォルトのパス等
///
pub struct ConfigLoader<C> {
    paths: Vec<ConfigPath>,
    _marker: PhantomData<C>,
}

impl<C> ConfigLoader<C> {
    pub fn new() -> Self {
        ConfigLoader {
            paths: vec![],
            _marker: PhantomData,
        }
    }

    pub fn add_source(mut self, path: ConfigPath) -> Self {
        self.paths.push(path);
        self
    }

    pub fn default_source(self, path: ConfigPath) -> LoaderWithDefault<C> {
        LoaderWithDefault {
            _loader: self,
            default_path: path,
        }
    }
}

pub struct LoaderWithDefault<C> {
    _loader: ConfigLoader<C>,
    default_path: ConfigPath,
}

impl<C> LoaderWithDefault<C> {
    pub fn load(self) -> (PathBuf, Result<C, C::ErrorType>)
    where
        C: ConfigTrait + Default,
    {
        // 読み込みオンリー
        // FIXME: 指定読み込み対象への処理をやる
        // self._loader.paths

        let path = self
            .default_path
            .to_path()
            .unwrap_or_else(|| panic!("Please check config file path! :{:?}", &self.default_path));
        // folder作成して
        let _ = fs::create_dir_all(path.parent().unwrap()).unwrap();

        if path.is_file() {
            info!("config file exists: {:?}", &path);
        } else {
            if let Err(e) = File::create(&path) {
                panic!("Can't create Config file. check here: {:?}", e);
            }
        };

        // ファイル読み込む
        let ret = C::load_file(&path);
        if let Err(e) = ret {
            panic!("can't write config error:{:?}", e);
        }
        (path, ret)
    }
}

/*
// MEMO: 設定ファイルを読み込む順序
// 1. 起動時に指定があればその設定ファイルを読み込む
//    - 読み込めなければエラーを出して終了
//    - 読み込み時に書き込み権限がなければ、ワーニングを出す
// 2. 指定が無ければ、実行ファイルと同一ディレクトリの設定ファイルを読み込む
//    - std::env::current_exe()
//    - ただしフォルダへの書き込み権限が無ければ、この操作は行わない
// 3. 設定ファイルが無ければ、個人フォルダの指定ディレクトリからファイルを読み込む
// 4. 設定ファイルが無ければ、個人フォルダの指定ディレクトリ、ファイルを作成、新規設定を読み込む

/// コンフィグを解析して読み込む
/// その際、add_sorceに指定された順番でファイルを読み込む。
/// 最初に存在したファイルを設定ファイルとして扱い、成功の可否にかかわらず読み込み、結果を返却する
// pub fn load(self) -> (Option<PathBuf>, Result<C, E>) {
// pub fn load(self) -> ConfigWithPath<C>
// where
//     C: ConfigTrait + Default,
// {
//     if self.paths.len() == 0 {
//         panic!("Please call add_source least once.")
//     }

//     let mut candiate_file_paths: Vec<_> = self
//         .paths
//         .iter()
//         .filter_map(|cpath| Some(cpath.to_path()?))
//         .collect();

//     let candidate = candiate_file_paths.iter().find_map(|path| {
//         if path.is_file() {
//             //
//             let config = C::load_file(path);
//             Some(config)
//         } else {
//             None
//         }
//     });

//     match candidate {
//         Some(_) => todo!(),
//         None => ConfigWithPath::NewConfigCandidate(C::default()),
//     }
// }
// enum ConfigWithPath<C>
// where
//     C: ConfigTrait,
// {
//     Exist(PathBuf, C),
//     ExistButError(PathBuf, C::ErrorType),
//     NewConfigCandidate(C),
// }
 */

#[cfg(test)]
mod t {
    use crate::config::Config;

    use super::*;
    //
    // ConfigLoader
    //
    #[ignore = "ちょっとまだ修してない"]
    #[test]
    fn test_loader() {
        // 正常系
        let config_file_path = "tests/files/config/peercast-rt.ini";
        let def_config_file_path = "src/config/config.example.ini";
        std::env::set_var("TEST_ENV_VAR1", config_file_path);

        let config = ConfigLoader::<Config>::new()
            .add_source(ConfigPath::Env("TEST_ENV_VAR1".into()))
            .default_source(ConfigPath::Path(def_config_file_path.into()))
            .load();

        let (load_file_path, c) = config;
        assert_eq!(load_file_path, PathBuf::from(config_file_path));
        assert!(c.is_ok());

        // 異常系
        // let config = ConfigLoader::<Config>::new()
        //     .add_source(ConfigPath::Env("NOT_FOUND11111".into()))
        //     .default_source(ConfigPath::Path("src/config.example.ini".into()))
        //     .load();
        // assert!(config.is_none());

        // // 異常系 + 正常系
        // let config = ConfigLoader::<Config>::new()
        //     .add_source(ConfigPath::Env("NOT_FOUND11111".into()))
        //     .add_source(ConfigPath::Path("src/config.example.ini".into()))
        //     .load();
        // let (p, c) = config.unwrap();
        // assert_eq!(p, PathBuf::from("src/config.example.ini"));
        // assert!(c.is_ok());

        // // 異常系 + 異常系 + 正常系
        // let config = ConfigLoader::<Config>::new()
        //     .add_source(ConfigPath::Env("NOT_FOUND11111".into()))
        //     .add_source(ConfigPath::Path("NOT_FOUND_PATH".into()))
        //     .add_source(ConfigPath::Path("src/config.example.ini".into()))
        //     .load();
        // let (p, c) = config.unwrap();
        // assert_eq!(p, PathBuf::from("src/config.example.ini"));
        // assert!(c.is_ok());
    }
}
