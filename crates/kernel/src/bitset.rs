type Block = u64;
pub struct BitSet<const N: usize>
where
    [Block; N.div_ceil(Block::BITS as usize)]: Sized,
{
    blocks: [Block; N.div_ceil(Block::BITS as usize)],
}

impl<const N: usize> Default for BitSet<N>
where
    [Block; N.div_ceil(Block::BITS as usize)]: Sized,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> BitSet<N>
where
    [Block; N.div_ceil(Block::BITS as usize)]: Sized,
{
    const ALLZERO: Block = 0;
    const ALLONE: Block = Block::MAX;
    const SHIFT: usize = Block::BITS.trailing_zeros() as usize;
    const MASK: usize = !((1 << Self::SHIFT) - 1);

    pub const fn new() -> Self {
        Self {
            blocks: [Self::ALLZERO; N.div_ceil(Block::BITS as usize)],
        }
    }

    fn unpack_index(i: usize) -> (usize, usize) {
        (i >> Self::SHIFT, i & Self::MASK)
    }
    fn canonicalize_range<R: core::ops::RangeBounds<usize>>(range: R) -> (usize, usize) {
        let s = match range.start_bound() {
            core::ops::Bound::Included(&i) => i,
            core::ops::Bound::Excluded(&i) => i + 1,
            core::ops::Bound::Unbounded => 0,
        };
        let t = match range.end_bound() {
            core::ops::Bound::Included(&i) => i + 1,
            core::ops::Bound::Excluded(&i) => i,
            core::ops::Bound::Unbounded => N,
        };
        assert!(0 <= s);
        assert!(s < t);
        assert!(t <= N);
        (s, t)
    }

    pub fn contains(&mut self, i: usize) -> bool {
        assert!(i < N);
        let (u, l) = Self::unpack_index(i);
        (self.blocks[u] >> l) & 1 == 1
    }

    pub fn insert(&mut self, i: usize) {
        assert!(i < N);
        let (u, l) = Self::unpack_index(i);
        self.blocks[u] |= 1 << l;
    }

    pub fn remove(&mut self, i: usize) {
        assert!(i < N);
        let (u, l) = Self::unpack_index(i);
        self.blocks[u] &= !(1 << l);
    }

    pub fn insert_range<R: core::ops::RangeBounds<usize>>(&mut self, range: R) {
        let (s, t) = Self::canonicalize_range(range);
        let (us, ls) = Self::unpack_index(s);
        let (ut, lt) = Self::unpack_index(t);
        if us == ut {
            self.blocks[us] |= (Self::ALLONE << ls) & !(Self::ALLONE << lt);
        } else {
            self.blocks[us] |= Self::ALLONE << ls;
            self.blocks[ut] |= !(Self::ALLONE << lt);
            self.blocks[(us + 1)..ut].fill(Self::ALLONE);
        }
    }

    pub fn remove_range<R: core::ops::RangeBounds<usize>>(&mut self, range: R) {
        let (s, t) = Self::canonicalize_range(range);
        let (us, ls) = Self::unpack_index(s);
        let (ut, lt) = Self::unpack_index(t);
        if us == ut {
            self.blocks[us] &= (!(Self::ALLONE << ls)) | (Self::ALLONE << lt);
        } else {
            self.blocks[us] &= !(Self::ALLONE << ls);
            self.blocks[ut] &= Self::ALLONE << lt;
            self.blocks[(us + 1)..ut].fill(Self::ALLZERO);
        }
    }
}
