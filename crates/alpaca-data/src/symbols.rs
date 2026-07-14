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
