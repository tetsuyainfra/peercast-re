use libpeercast_re::GnuId;
use peercast_root::{
    connection::{RootConnectionFactory, RootConnectionManager},
    repository::{RootChannelFactory, RootChannelRepository},
    service::yellow_page::YellowPageService,
};

use crate::app::cli;

////////////////////////////////////////////////////////////////////////////////
/// AppState
///
#[derive(Debug)]
pub struct AppState {
    pub self_session_id: GnuId,
    pub api_config: ApiConfig,
    //     pub index_txt_footer: Vec<IndexInfo>,
    pub db_pool: sqlx::Pool<sqlx::sqlite::Sqlite>,
    //
    pub channel: ChannelRepo,
    pub yellow_page: YellowPageService,
    //
    pub conns: ConnectionRepo,
}

impl From<AppState> for ArcState {
    fn from(app_state: AppState) -> Self {
        ArcState(std::sync::Arc::new(app_state))
    }
}

/// AppStateをArcで包んだもの。AppStateは複数のハンドラーから共有されるため、Arcで包む。
#[derive(Debug, Clone)]
pub struct ArcState(pub std::sync::Arc<AppState>);

impl std::ops::Deref for ArcState {
    type Target = AppState;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

////////////////////////////////////////////////////////////////////////////////
/// ApiConfig
///
#[derive(Debug)]
pub struct ApiConfig {
    /// HTMLファイルを外部から提供するディレクトリ。Noneの場合は組み込みのHTMLを使用
    pub use_outer_html_dir: Option<std::path::PathBuf>,

    // 埋め込みテンプレートに渡す定数値
    pub embed_tmpl_ctx: EmbedTemplateCtx,

    /// CORS許可リスト
    pub allow_cors: Vec<String>,

    /// APIが返すデータのキャッシュの最大有効期限（秒）
    pub cache_max_age: u32,

    /// クライアントIPの取得元
    pub client_ip_source: cli::ClientIpSourceArg,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, Clone)]
pub struct EmbedTemplateCtx {
    pub EMBED_TITLE: String,
    pub EMBED_YP_NAME: String,
    pub EMBED_URL_HTTP: String,
    pub EMBED_URL_PCP: String,

    /// トラッカーのPeerCastでジャンルに含ませるべき文字列
    pub tracker_yp_name: String,
}

////////////////////////////////////////////////////////////////////////////////
/// ChannelRepository/ChannelFactory用のstructの定義
///
#[derive(Debug)]
pub struct ChannelRepo {
    pub repository: RootChannelRepository,
    pub factory: RootChannelFactory,
}

////////////////////////////////////////////////////////////////////////////////
/// ConnectionManager/ConnectionFactory用のstructの定義
///
#[derive(Debug)]
pub struct ConnectionRepo {
    pub manager: RootConnectionManager,
    pub factory: RootConnectionFactory,
}
