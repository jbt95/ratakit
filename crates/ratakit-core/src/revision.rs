/// Types that expose a monotonically increasing revision for cache invalidation.
pub trait Revisioned {
    fn revision(&self) -> u64;
}
