use serde_json::{json, Value};

#[cfg(test)]
mod tests {
    use super::{
        allowed_tools, anthropic_tool_schemas, openai_tool_schemas,
        openai_tool_schemas_for_runtime,
    };
    use crate::settings::SettingsService;

    #[test]
    fn only_whitelists_safe_tools() {
        let tools = allowed_tools();
        assert!(tools.contains(&"market_data".to_string()));
        assert!(tools.contains(&"stock_info".to_string()));
        assert!(tools.contains(&"bid_ask".to_string()));
        assert!(tools.contains(&"kline_data".to_string()));
        assert!(tools.contains(&"financial_report_info".to_string()));
        assert!(!tools.contains(&"spread_analyzer".to_string()));
        assert!(!tools.contains(&"coin_info".to_string()));
        assert!(!tools.contains(&"real_order_execution".to_string()));
    }

    #[test]
    fn builds_tool_schemas_for_both_provider_shapes() {
        assert!(!openai_tool_schemas().is_empty());
        assert!(!anthropic_tool_schemas().is_empty());
    }

    #[test]
    fn tool_schemas_use_a_share_language() {
        let schemas = serde_json::to_string(&openai_tool_schemas()).unwrap();

        assert!(schemas.contains("stockCode"));
        assert!(schemas.contains("沪深 A 股"));
        assert!(!schemas.contains("marketType"));
        assert!(!schemas.contains("funding"));
        assert!(!schemas.contains("perpetual"));
        assert!(!schemas.contains("spot"));
        assert!(!schemas.contains("coin"));
        assert!(!schemas.contains("crypto"));
    }

    #[test]
    fn financial_report_tool_schema_follows_runtime_setting() {
        let settings = SettingsService::default();
        let mut runtime = settings.get_runtime_settings();
        runtime.use_financial_report_data = false;
        let disabled = serde_json::to_string(&openai_tool_schemas_for_runtime(&runtime)).unwrap();
        assert!(!disabled.contains("financial_report_info"));

        runtime.use_financial_report_data = true;
        let enabled = serde_json::to_string(&openai_tool_schemas_for_runtime(&runtime)).unwrap();
        assert!(enabled.contains("financial_report_info"));
        assert!(enabled.contains("财报 AI 分析结论"));
        assert!(enabled.contains("雷达评分"));
        assert!(!enabled.contains("原始财报数据"));
    }
}

pub fn allowed_tools() -> Vec<String> {
    tool_specs(true)
        .iter()
        .map(|spec| spec.name.to_string())
        .collect()
}

pub fn openai_tool_schemas() -> Vec<Value> {
    tool_specs(true)
        .iter()
        .map(|spec| {
            json!({
                "type": "function",
                "function": {
                    "name": spec.name,
                    "description": spec.description,
                    "parameters": spec.parameters,
                }
            })
        })
        .collect()
}

pub fn openai_tool_schemas_for_runtime(runtime: &crate::models::RuntimeSettingsDto) -> Vec<Value> {
    tool_specs(runtime.use_financial_report_data)
        .iter()
        .map(|spec| {
            json!({
                "type": "function",
                "function": {
                    "name": spec.name,
                    "description": spec.description,
                    "parameters": spec.parameters,
                }
            })
        })
        .collect()
}

pub fn anthropic_tool_schemas() -> Vec<Value> {
    tool_specs(true)
        .iter()
        .map(|spec| {
            json!({
                "name": spec.name,
                "description": spec.description,
                "input_schema": spec.parameters,
            })
        })
        .collect()
}

pub fn anthropic_tool_schemas_for_runtime(runtime: &crate::models::RuntimeSettingsDto) -> Vec<Value> {
    tool_specs(runtime.use_financial_report_data)
        .iter()
        .map(|spec| {
            json!({
                "name": spec.name,
                "description": spec.description,
                "input_schema": spec.parameters,
            })
        })
        .collect()
}

struct ToolSpec {
    name: &'static str,
    description: &'static str,
    parameters: Value,
}

fn tool_specs(include_financial_report: bool) -> Vec<ToolSpec> {
    let mut specs = vec![
        ToolSpec {
            name: "market_data",
            description: "读取沪深 A 股自选股缓存行情，包括股票代码、名称、最新价、涨跌幅、成交量、成交额和缓存时间；不主动刷新 AKShare。",
            parameters: json!({
                "type": "object",
                "properties": {
                    "stockCode": { "type": "string", "description": "A 股代码，例如 600000、SHSE.600000 或 SZSE.000001" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 10 }
                },
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "portfolio",
            description: "读取本地模拟投资组合模式、模拟资金账户、模拟持仓和模拟委托快照。",
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "positions",
            description: "读取当前 A 股模拟持仓和近期模拟委托，可按股票代码或模拟资金账户过滤。",
            parameters: json!({
                "type": "object",
                "properties": {
                    "stockCode": { "type": "string", "description": "A 股代码，例如 600000、SHSE.600000 或 SZSE.000001" },
                    "account": { "type": "string" },
                    "paperOnly": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "recommendation_history",
            description: "读取最新投资建议和近期 A 股建议记录，不刷新行情或重新生成建议。",
            parameters: json!({
                "type": "object",
                "properties": {
                    "stockCode": { "type": "string", "description": "A 股代码，例如 600000、SHSE.600000 或 SZSE.000001" },
                    "latestOnly": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "stock_info",
            description: "通过 AKShare stock_individual_basic_info_xq 读取 A 股上市公司基础资料。",
            parameters: json!({
                "type": "object",
                "properties": {
                    "stockCode": { "type": "string", "description": "A 股代码，例如 600000、SHSE.600000 或 SZSE.000001" }
                },
                "required": ["stockCode"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "risk_calculator",
            description: "汇总最新或指定 A 股投资建议的风险状态、止损、最大亏损估计和失效条件；不会创建委托。",
            parameters: json!({
                "type": "object",
                "properties": {
                    "stockCode": { "type": "string", "description": "A 股代码，例如 600000、SHSE.600000 或 SZSE.000001" }
                },
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "paper_order_draft",
            description: "基于最新可执行 A 股投资建议创建本地模拟委托草稿。",
            parameters: json!({
                "type": "object",
                "properties": {
                    "account": { "type": "string" }
                },
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "signal_scan",
            description: "使用 AKShare K 线对自选 A 股执行信号扫描。可指定单只股票，返回原始策略信号和统一信号结果。",
            parameters: json!({
                "type": "object",
                "properties": {
                    "stockCode": { "type": "string", "description": "A 股代码，例如 600000、SHSE.600000 或 SZSE.000001" }
                },
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "bid_ask",
            description: "通过 AKShare stock_bid_ask_em 读取 A 股实时五档买卖盘、最新价、涨跌幅、成交量和涨跌停价。",
            parameters: json!({
                "type": "object",
                "properties": {
                    "stockCode": { "type": "string", "description": "A 股代码，例如 600000、SHSE.600000 或 SZSE.000001" }
                },
                "required": ["stockCode"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "kline_data",
            description: "通过 AKShare 为 Assistant 返回 5m、1h、1d、1w 四个周期的 A 股 K 线数据。",
            parameters: json!({
                "type": "object",
                "properties": {
                    "stockCode": { "type": "string", "description": "A 股代码，例如 600000、SHSE.600000 或 SZSE.000001" },
                    "count": { "type": "integer", "minimum": 1, "maximum": 120 }
                },
                "required": ["stockCode"],
                "additionalProperties": false
            }),
        },
    ];
    if include_financial_report {
        specs.push(ToolSpec {
            name: "financial_report_info",
            description: "读取本地缓存的 A 股财报 AI 分析结论和雷达评分，不触发 AKShare 拉取或 AI 分析。",
            parameters: json!({
                "type": "object",
                "properties": {
                    "stockCode": { "type": "string", "description": "A 股代码，例如 600000、SHSE.600000 或 SZSE.000001" }
                },
                "required": ["stockCode"],
                "additionalProperties": false
            }),
        });
    }
    specs
}
