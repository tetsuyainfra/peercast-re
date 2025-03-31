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
    fn load_file(path: &PathBuf) -> Result<Self, Self::ErrorType>;
    fn save_file(&self, path: &PathBuf) -> Result<(), Self::ErrorType>;
}

/// ConfigLoaderは指定された設定ファイルを順次読み込み、指定された設定ファイル構造体Tを返すローダーである。
///
/// env_or_args() は値を指定していれば、読み込みエラーの場合は後続のコンフィグファイルは読み込まず、エラーを返す
/// add_source() は読み込みはするが、エラーの場合無視をする。
/// default_source() は必ず指定しなくてはならず、エラーの場合はpanicする
pub struct ConfigLoader<C> {
    _env_or_args: Option<PathBuf>,
    paths: Vec<PathBuf>,
    _marker: PhantomData<C>,
}

impl<C> ConfigLoader<C> {
    pub fn new() -> Self {
        ConfigLoader {
            _env_or_args: None,
            paths: vec![],
            _marker: PhantomData,
        }
    }

    pub fn env_or_args(mut self, path: Option<PathBuf>) -> Self {
        self._env_or_args = path;
        self
    }

    pub fn add_source(mut self, path: PathBuf) -> Self {
        self.paths.push(path);
        self
    }

    pub fn default_source(self, default_path: PathBuf) -> LoaderWithDefault<C> {
        LoaderWithDefault {
            _loader: self,
            default_path,
        }
    }
}

pub struct LoaderWithDefault<C> {
    _loader: ConfigLoader<C>,
    default_path: PathBuf,
}

impl<C> LoaderWithDefault<C>
where
    C: ConfigTrait + Default,
{
    pub fn load(self) -> (PathBuf, Result<C, C::ErrorType>) {
        // Env or Args
        if self._loader._env_or_args.is_some() {
            let path = self._loader._env_or_args.unwrap();
            let ret = C::load_file(&path);
            return (path, ret);
        };

        // additional path
        for path in self._loader.paths {
            let ret = C::load_file(&path);
            if ret.is_ok() {
                return (path, ret);
            }
        }

        // Default Path
        let path = self.default_path;
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
        match ret {
            Err(e) => {
                panic!("can't write config error:{:?}", e);
            }
            Ok(config) => {
                //
                config.save_file(&path);
                (path, Ok(config))
            }
        }
    }
}

#[cfg(test)]
mod t {
    use crate::config::Config;

    use super::*;

    const CONFIG_FILE_PATH: &str = "tests/files/config/peercast-rt.ini";
    const DEFAULT_FILE_PATH: &str = "src/config/config.example.ini";
    const SYNTAX_ERROR_FILE_PATH: &str = "tests/files/config/peercast-rt_error.ini";

    //
    // ConfigLoader
    //
    #[test]
    fn test_loader() {
        use clap::Parser;
        #[derive(Debug, Parser)]
        #[command()]
        struct Command {
            #[clap(
                short = 'C',
                long = "config",
                value_name = "CONFIG_FILE",
                env = "TEST_ENV_VAR1"
            )]
            config_file: Option<PathBuf>,
        }
        // envから読み取る
        std::env::set_var("TEST_ENV_VAR1", DEFAULT_FILE_PATH);
        let args = vec!["command"];
        let cmd = Command::parse_from(args.iter());
        assert_eq!(cmd.config_file, Some(DEFAULT_FILE_PATH.into()));

        // コンフィグファイルの読み込み
        let (path, ret_config) = ConfigLoader::<Config>::new()
            .env_or_args(cmd.config_file.clone()) // envがSomeなので読み込みを試すことになる
            .add_source(PathBuf::from("peercast-re.ini")) // ファイルが有れば読み込む
            .default_source(DEFAULT_FILE_PATH.into()) // 標準の設定ファイル
            .load();
        assert_eq!(path, cmd.config_file.unwrap());
        assert!(ret_config.is_ok());
    }

    #[test]
    fn test_env_args() {
        std::env::set_var("TEST_ENV_VAR1", CONFIG_FILE_PATH);

        // 正常系
        let (path, ret_config) = ConfigLoader::<Config>::new()
            .env_or_args(Some(PathBuf::from(CONFIG_FILE_PATH)))
            .default_source(DEFAULT_FILE_PATH.into())
            .load();
        assert_eq!(path, PathBuf::from(CONFIG_FILE_PATH));
        assert!(ret_config.is_ok());

        // 異常系
        let (path, ret_config) = ConfigLoader::<Config>::new()
            .env_or_args(Some(PathBuf::from("./")))
            .default_source(DEFAULT_FILE_PATH.into())
            .load();
        assert_eq!(path, PathBuf::from("./"));
        assert!(ret_config.is_err());
    }

    #[test]
    fn test_add_source() {
        // 正常系
        let (path, ret_config) = ConfigLoader::<Config>::new()
            .env_or_args(None)
            .add_source(CONFIG_FILE_PATH.into())
            .default_source(DEFAULT_FILE_PATH.into())
            .load();
        assert_eq!(path, PathBuf::from(CONFIG_FILE_PATH));
        assert!(ret_config.is_ok());

        // 異常系(デフォルトソースにフォールバック)
        let (path, ret_config) = ConfigLoader::<Config>::new()
            .env_or_args(None)
            .add_source("./".into())
            .default_source(DEFAULT_FILE_PATH.into())
            .load();
        assert_eq!(path, PathBuf::from(DEFAULT_FILE_PATH));
        assert!(ret_config.is_ok());
    }

    #[test]
    fn test_default_source() {
        // 正常系
        let (path, ret_config) = ConfigLoader::<Config>::new()
            .env_or_args(None)
            // .add_source("./") // 無し
            .default_source(DEFAULT_FILE_PATH.into())
            .load();
        assert_eq!(path, PathBuf::from(DEFAULT_FILE_PATH));
        assert!(ret_config.is_ok());
    }

    #[ignore = "this test occur panic"]
    #[test]
    #[should_panic]
    fn test_default_source_with_panic() {
        // 異常系(読み込みエラー ファイルSyntax異常)
        let (path, ret_config) = ConfigLoader::<Config>::new()
            .env_or_args(None)
            // .add_source("./")
            .default_source(SYNTAX_ERROR_FILE_PATH.into())
            .load();
        assert_eq!(path, PathBuf::from(SYNTAX_ERROR_FILE_PATH));
        assert!(ret_config.is_err());
    }

    #[test]
    fn test_clap() {
        // Clapの動作について
        use clap::Parser;
        #[derive(Debug, Parser)]
        #[command()]
        struct Command {
            #[clap(
                short = 'C',
                long = "config",
                value_name = "CONFIG_FILE",
                env = "TEST_ENV_VAR1"
            )]
            config_file: Option<PathBuf>,
        }

        // -Cの変数を読み取る
        let args = vec!["command", "-C", CONFIG_FILE_PATH];
        let x = Command::parse_from(args.iter());
        assert_eq!(x.config_file, Some(CONFIG_FILE_PATH.into()));

        // 環境変数の値を読み取る
        std::env::set_var("TEST_ENV_VAR1", DEFAULT_FILE_PATH);
        let args = vec!["command"];
        let x = Command::parse_from(args.iter());
        assert_eq!(x.config_file, Some(DEFAULT_FILE_PATH.into()));

        // 環境変数を指定していても-Cの変数を優先する
        std::env::set_var("TEST_ENV_VAR1", DEFAULT_FILE_PATH);
        let args = vec!["command", "-C", CONFIG_FILE_PATH];
        let x = Command::parse_from(args.iter());
        assert_eq!(x.config_file, Some(CONFIG_FILE_PATH.into()));
    }
}
