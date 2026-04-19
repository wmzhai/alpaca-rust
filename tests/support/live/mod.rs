#![allow(dead_code, unused_imports)]

pub mod env;
pub mod error;
pub mod http;
pub mod options;
pub mod paper;
pub mod recording;

pub use env::{
    AlpacaService, DATA_API_KEY_ENV, DATA_SECRET_KEY_ENV, DEFAULT_SAMPLE_ROOT_DIR,
    DEFAULT_TRADE_BASE_URL, DataServiceConfig, LEGACY_KEY_ENV, LEGACY_SECRET_ENV, LiveTestEnv,
    RECORD_SAMPLES_ENV, SAMPLE_ROOT_ENV, TRADE_API_KEY_ENV, TRADE_BASE_URL_ENV,
    TRADE_SECRET_KEY_ENV, TradeServiceConfig, workspace_root_from_manifest_dir,
};
pub use error::SupportError;
pub use http::{JsonProbeResponse, LiveHttpProbe};
pub use options::{
    DayWindow, ObservedOptionContract, OptionContractType, discover_active_option_contract,
    discover_option_contracts, full_day_window_from_timestamp, parse_occ_option_symbol,
};
pub use paper::{
    PaperClock, PaperSessionState, can_submit_live_paper_orders, fetch_paper_clock,
    paper_market_session_state, trading_day_from_timestamp,
};
pub use recording::SampleRecorder;
