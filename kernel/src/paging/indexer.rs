
pub struct PageMapIndexer{
    pub pdp: usize,
    pub pd: usize,
    pub pt: usize,
    pub p: usize,
}

impl PageMapIndexer{
    pub fn for_addr(addr: usize) -> Self{
        let mut virtualAddress = addr;
        virtualAddress >>= 12;
        let p = virtualAddress & 0x1ff;
        virtualAddress >>= 9;
        let pt = virtualAddress & 0x1ff;
        virtualAddress >>= 9;
        let pd = virtualAddress & 0x1ff;
        virtualAddress >>= 9;
        let pdp = virtualAddress & 0x1ff;
        virtualAddress >>= 9;

        Self{
            pdp, pd, pt, p
        }
    }

}