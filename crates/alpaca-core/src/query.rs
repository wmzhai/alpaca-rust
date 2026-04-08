#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct QueryWriter {
    pairs: Vec<(String, String)>,
}

impl QueryWriter {
    pub fn push<T>(&mut self, key: &'static str, value: T)
    where
        T: ToString,
    {
        self.pairs.push((key.to_owned(), value.to_string()));
    }

    pub fn push_opt<T>(&mut self, key: &'static str, value: Option<T>)
    where
        T: ToString,
    {
        if let Some(value) = value {
            self.push(key, value);
        }
    }

    pub fn push_csv<I, T>(&mut self, key: &'static str, values: I)
    where
        I: IntoIterator<Item = T>,
        T: ToString,
    {
        let value = values
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",");

        if !value.is_empty() {
            self.push(key, value);
        }
    }

    #[must_use]
    pub fn finish(self) -> Vec<(String, String)> {
        self.pairs
    }
}
