trait BlockExt
where
    Self: Sized + core::ops::Shl<usize, Output = Self>,
{
    const BITS: usize;
    const ONE: Self;
    const NOBIT: Self;
    const ALLBITS: Self;
    const SHIFT: usize = Self::BITS.trailing_zeros() as usize;
    const MASK: usize = (1 << Self::SHIFT) - 1;

    fn get_bit(index: usize) -> Self {
        Self::ONE << index
    }
}

type Block = u64;
impl BlockExt for Block {
    const BITS: usize = Self::BITS as usize;
    const ONE: Self = 1;
    const NOBIT: Self = 0;
    const ALLBITS: Self = u64::MAX;
}

pub struct BitSet<const N: usize>
where
    [(); 1 + N.div_ceil(Block::BITS as usize)]: Sized,
{
    blocks: [Block; 1 + N.div_ceil(Block::BITS as usize)],
}

impl<const N: usize> Default for BitSet<N>
where
    [(); 1 + N.div_ceil(Block::BITS as usize)]: Sized,
{
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl<const N: usize> BitSet<N>
where
    [(); 1 + N.div_ceil(Block::BITS as usize)]: Sized,
{
    pub const fn new() -> Self {
        Self {
            blocks: [Block::NOBIT; 1 + N.div_ceil(Block::BITS as usize)],
        }
    }

    fn unpack_index(i: usize) -> (usize, usize) {
        (i >> Block::SHIFT, i & Block::MASK)
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
        assert!(s <= t);
        assert!(t <= N);
        (s, t)
    }

    pub fn contains(&mut self, i: usize) -> bool {
        assert!(i < N);
        let (u, l) = Self::unpack_index(i);
        self.blocks[u] & Block::get_bit(l) != Block::NOBIT
    }

    pub fn insert(&mut self, i: usize) {
        assert!(i < N);
        let (u, l) = Self::unpack_index(i);
        self.blocks[u] |= Block::get_bit(l);
    }

    pub fn remove(&mut self, i: usize) {
        assert!(i < N);
        let (u, l) = Self::unpack_index(i);
        self.blocks[u] &= !Block::get_bit(l);
    }

    pub fn insert_range<R: core::ops::RangeBounds<usize>>(&mut self, range: R) {
        let (s, t) = Self::canonicalize_range(range);
        let (us, ls) = Self::unpack_index(s);
        let (ut, lt) = Self::unpack_index(t);
        if us == ut {
            self.blocks[us] |= (Block::ALLBITS << ls) & !(Block::ALLBITS << lt);
        } else {
            self.blocks[us] |= Block::ALLBITS << ls;
            self.blocks[ut] |= !(Block::ALLBITS << lt);
            self.blocks[(us + 1)..ut].fill(Block::ALLBITS);
        }
    }

    pub fn remove_range<R: core::ops::RangeBounds<usize>>(&mut self, range: R) {
        let (s, t) = Self::canonicalize_range(range);
        let (us, ls) = Self::unpack_index(s);
        let (ut, lt) = Self::unpack_index(t);
        if us == ut {
            self.blocks[us] &= (!(Block::ALLBITS << ls)) | (Block::ALLBITS << lt);
        } else {
            self.blocks[us] &= !(Block::ALLBITS << ls);
            self.blocks[ut] &= Block::ALLBITS << lt;
            self.blocks[(us + 1)..ut].fill(Block::NOBIT);
        }
    }
}
