/// One item paired across the two files, or found in only one of them.
pub enum Pair<T> {
    Both(T, T),
    OnlyA(T),
    OnlyB(T),
}

impl<T> Pair<T> {
    /// The item from a if present, otherwise from b.
    pub fn either(&self) -> &T {
        match self {
            Pair::Both(x, _) | Pair::OnlyA(x) | Pair::OnlyB(x) => x,
        }
    }

    pub fn as_ref(&self) -> Pair<&T> {
        match self {
            Pair::Both(a, b) => Pair::Both(a, b),
            Pair::OnlyA(a) => Pair::OnlyA(a),
            Pair::OnlyB(b) => Pair::OnlyB(b),
        }
    }

    pub fn map<U>(self, f: impl Fn(T) -> U) -> Pair<U> {
        match self {
            Pair::Both(a, b) => Pair::Both(f(a), f(b)),
            Pair::OnlyA(a) => Pair::OnlyA(f(a)),
            Pair::OnlyB(b) => Pair::OnlyB(f(b)),
        }
    }
}

/// Pairs items for which `same` holds, in order of occurrence.
pub fn pair_by<'x, T>(a: &'x [T], b: &'x [T], same: impl Fn(&T, &T) -> bool) -> Vec<Pair<&'x T>> {
    let mut used = vec![false; b.len()];
    let mut pairs = Vec::with_capacity(a.len());
    // Search from just after the previous match, so files in the same order pair in linear time.
    let mut cursor = 0;
    for x in a {
        match (cursor..b.len())
            .chain(0..cursor)
            .find(|&i| !used[i] && same(x, &b[i]))
        {
            Some(i) => {
                used[i] = true;
                cursor = i + 1;
                pairs.push(Pair::Both(x, &b[i]));
            }
            None => pairs.push(Pair::OnlyA(x)),
        }
    }
    pairs.extend(
        b.iter()
            .zip(used)
            .filter(|(_, used)| !used)
            .map(|(y, _)| Pair::OnlyB(y)),
    );
    pairs
}
