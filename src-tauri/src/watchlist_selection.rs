use std::collections::HashSet;

pub fn normalize_selected_symbols(selected_symbols: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    selected_symbols
        .iter()
        .map(|symbol| normalize_symbol(symbol))
        .filter(|symbol| !symbol.is_empty() && seen.insert(symbol.clone()))
        .collect()
}

pub fn normalize_selected_watchlist(
    selected_symbols: &[String],
    watchlist_symbols: &[String],
) -> Vec<String> {
    let selected: HashSet<String> = normalize_selected_symbols(selected_symbols)
        .into_iter()
        .collect();
    let mut seen = HashSet::new();
    watchlist_symbols
        .iter()
        .map(|symbol| normalize_symbol(symbol))
        .filter(|symbol| selected.contains(symbol) && seen.insert(symbol.clone()))
        .collect()
}

fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::{normalize_selected_symbols, normalize_selected_watchlist};

    #[test]
    fn keeps_selected_watchlist_symbols_in_watchlist_order() {
        let selected = vec![
            "szse.000001".into(),
            "shse.600000".into(),
            "szse.000001".into(),
        ];
        let watchlist = vec![
            "SHSE.600000".into(),
            "SZSE.000001".into(),
            "SHSE.600519".into(),
        ];

        assert_eq!(
            normalize_selected_watchlist(&selected, &watchlist),
            vec!["SHSE.600000".to_string(), "SZSE.000001".to_string()]
        );
    }

    #[test]
    fn deduplicates_selected_symbols() {
        let selected = vec!["  shse.600000 ".into(), "SHSE.600000".into(), "".into()];

        assert_eq!(
            normalize_selected_symbols(&selected),
            vec!["SHSE.600000".to_string()]
        );
    }
}
