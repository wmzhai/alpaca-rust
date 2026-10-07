const OCC_SUFFIX_LEN: usize = 15;
const DOTTED_SHARE_CLASSES: [(&str, &str); 2] = [("BRKA", "BRK.A"), ("BRKB", "BRK.B")];

pub fn options_underlying_symbol(input: &str) -> String {
    let normalized = normalized_code(input);
    if normalized.is_empty() {
        return normalized;
    }

    let root = occ_contract_root(&normalized).unwrap_or(normalized.as_str());
    // Adjusted deliveries keep a trailing digit on the OCC root (ETHA1). The
    // listed stock symbol is the root without that digit.
    let stripped = root.trim_end_matches(|ch: char| ch.is_ascii_digit());
    let stock_root = if stripped.is_empty() { root } else { stripped };
    stock_root.replace('.', "")
}

pub fn display_stock_symbol(input: &str) -> String {
    let underlying = options_underlying_symbol(input);
    if underlying.is_empty() {
        return underlying;
    }

    DOTTED_SHARE_CLASSES
        .iter()
        .find_map(|(provider_symbol, display_symbol)| {
            (*provider_symbol == underlying).then(|| (*display_symbol).to_owned())
        })
        .unwrap_or(underlying)
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
    use super::{display_stock_symbol, options_underlying_symbol};

    #[test]
    fn adjusted_option_root_uses_the_stock_symbol() {
        assert_eq!(display_stock_symbol("ETHA1"), "ETHA");
        assert_eq!(display_stock_symbol("ETHA1261016P00020000"), "ETHA");
        assert_eq!(display_stock_symbol("SOUN2"), "SOUN");
        assert_eq!(display_stock_symbol("BRKB1"), "BRK.B");
        assert_eq!(display_stock_symbol("ETHA"), "ETHA");
        assert_eq!(display_stock_symbol("BRK.B"), "BRK.B");
        assert_eq!(options_underlying_symbol("ETHA1"), "ETHA");
        assert_eq!(options_underlying_symbol("ETHA1261016P00020000"), "ETHA");
    }
}
