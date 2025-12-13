use reqwest::Url;

pub trait UrlExt {
    fn join_unchecked(&self, input: &str) -> Self;
}

impl UrlExt for Url {
    fn join_unchecked(&self, input: &str) -> Self {
        self.join(input).unwrap()
    }
}
