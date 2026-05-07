use crate::models::{
    ExchangeConnectionTestResultDto, ExchangeCredentialSummary, PaperOrderRowDto,
    PortfolioOverviewDto, PositionDto,
};
use crate::settings::{ExchangeSecretMaterial, SettingsService};

#[derive(Clone, Default)]
pub struct RealAccountService;

impl RealAccountService {
    pub async fn inspect_exchange_credentials(
        &self,
        _settings_service: &SettingsService,
    ) -> Vec<ExchangeCredentialSummary> {
        Vec::new()
    }

    pub async fn test_exchange_connection(
        &self,
        _credentials: ExchangeSecretMaterial,
    ) -> ExchangeConnectionTestResultDto {
        ExchangeConnectionTestResultDto {
            status: "disabled".into(),
            permission_read: false,
            permission_trade: false,
            permission_withdraw: false,
            message: "当前版本仅支持本地模拟账号，不连接真实交易所。".into(),
        }
    }

    pub async fn build_overview(
        &self,
        _settings_service: &SettingsService,
    ) -> anyhow::Result<PortfolioOverviewDto> {
        Ok(PortfolioOverviewDto {
            total_equity_usdt: 0.0,
            total_market_value_usdt: 0.0,
            total_pnl_usdt: 0.0,
            daily_pnl_usdt: 0.0,
            daily_pnl_percent: 0.0,
            account_mode: "paper".into(),
            risk_summary: "当前版本仅支持本地模拟账号。".into(),
            exchanges: Vec::new(),
        })
    }

    pub async fn build_positions(
        &self,
        _settings_service: &SettingsService,
    ) -> anyhow::Result<Vec<PositionDto>> {
        Ok(Vec::new())
    }

    pub async fn build_orders(
        &self,
        _settings_service: &SettingsService,
    ) -> anyhow::Result<Vec<PaperOrderRowDto>> {
        Ok(Vec::new())
    }
}
