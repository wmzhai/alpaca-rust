const OCC_SUFFIX_LEN: usize = 15;
const DOTTED_SHARE_CLASSES: [(&str, &str); 2] = [("BRKA", "BRK.A"), ("BRKB", "BRK.B")];

pub fn options_underlying_symbol(input: &str) -> String {
    let normalized = normalized_code(input);
    if normalized.is_empty() {
        return normalized;
    }

    let root = occ_contract_root(&normalized).unwrap_or(normalized.as_str());
    root.replace('.', "")
}

pub fn display_stock_symbol(input: &str) -> String {
    let normalized = normalized_code(input);
    if normalized.is_empty() {
        return normalized;
    }

    let root = occ_contract_root(&normalized).unwrap_or(normalized.as_str());
    DOTTED_SHARE_CLASSES
        .iter()
        .find_map(|(provider_symbol, display_symbol)| {
            (*provider_symbol == root).then(|| (*display_symbol).to_owned())
        })
        .unwrap_or_else(|| root.to_owned())
}

pub(crate) fn option_contract_symbol(input: &str) -> String {
    normalized_code(input)
}

fn normalized_code(input: &str) -> String {
    input.trim().to_uppercase().replace('/', ".")
}

fn occ_contract_root(value: &str) -> Option<&str> {
    is_occ_contract_symbol(value).then(|| &value[..value.len() - OCC_SUFFIX_LEN])
}

fn is_occ_contract_symbol(value: &str) -> bool {
    if value.len() <= OCC_SUFFIX_LEN {
        return false;
    }

    let suffix = &value[value.len() - OCC_SUFFIX_LEN..];
    suffix[..6].chars().all(|value| value.is_ascii_digit())
        && matches!(&suffix[6..7], "C" | "P")
        && suffix[7..].chars().all(|value| value.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{display_stock_symbol, option_contract_symbol, options_underlying_symbol};

    #[test]
    fn normalizes_option_underlying_symbols_and_occ_roots() {
        assert_eq!(options_underlying_symbol(" brk.b "), "BRKB");
        assert_eq!(options_underlying_symbol("brk/b"), "BRKB");
        assert_eq!(options_underlying_symbol("BRKB250620P00480000"), "BRKB");
        assert_eq!(options_underlying_symbol("aapl"), "AAPL");
    }

    #[test]
    fn restores_display_stock_symbol_for_supported_share_classes() {
        assert_eq!(display_stock_symbol("BRKB"), "BRK.B");
        assert_eq!(display_stock_symbol("brk/b"), "BRK.B");
        assert_eq!(display_stock_symbol("BRKB250620P00480000"), "BRK.B");
        assert_eq!(display_stock_symbol("AAPL"), "AAPL");
    }

    #[test]
    fn normalizes_contract_symbols_without_creating_parallel_variants() {
        assert_eq!(
            option_contract_symbol(" brkb250620p00480000 "),
            "BRKB250620P00480000"
        );
    }
}
