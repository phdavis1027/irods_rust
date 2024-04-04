pub trait BorrowingKeyValPair<'s, K, V> {
    fn add_kw(&mut self, kw: K, val: &'s V);

    fn set_kw(&mut self, kw: K);

    fn get_kw(&self, kw: K) -> Option<&V>;

    fn keys_iter<It>(&self) -> It
    where
        It: Iterator<Item = K>;
}
