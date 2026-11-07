//! Memory pooling.
//!
//! Pre-allocated buffers and object pools to avoid allocations during
//! simulation steps, keeping frame times stable.


/// A reusable object pool for a generic type.
#[derive(Clone, Debug, Default)]
pub struct ObjectPool<T: Clone + Default> {
    pub available: Vec<T>,
    pub in_use: usize,
    pub total_allocated: u64,
}

impl<T: Clone + Default> ObjectPool<T> {
    pub fn new() -> Self {
        Self::default()
    }
    /// Acquire an object from the pool.
    pub fn acquire(&mut self) -> T {
        if let Some(obj) = self.available.pop() {
            self.in_use += 1;
            obj
        } else {
            self.total_allocated += 1;
            self.in_use += 1;
            T::default()
        }
    }
    /// Return an object to the pool.
    pub fn release(&mut self, obj: T) {
        self.available.push(obj);
        self.in_use = self.in_use.saturating_sub(1);
    }
    pub fn in_use_count(&self) -> usize {
        self.in_use
    }
    pub fn available_count(&self) -> usize {
        self.available.len()
    }
}

/// A fixed-size buffer pool for byte buffers.
#[derive(Clone, Debug)]
pub struct BufferPool {
    pub pools: Vec<(usize, Vec<Vec<u8>>)>, // (block_size, available buffers)
    pub total_allocated_bytes: u64,
}

impl Default for BufferPool {
    fn default() -> Self {
        Self::new()
    }
}

impl BufferPool {
    pub fn new() -> Self {
        Self {
            pools: Vec::new(),
            total_allocated_bytes: 0,
        }
    }
    /// Acquire a buffer of at least `size` bytes.
    pub fn acquire(&mut self, size: usize) -> Vec<u8> {
        // Find best-fit pool
        let best_idx = self.pools.iter().position(|(s, _)| *s >= size);
        if let Some(idx) = best_idx {
            if let Some(buf) = self.pools[idx].1.pop() {
                return buf;
            }
        }
        // No available buffer, allocate new
        let size = size.next_power_of_two();
        self.total_allocated_bytes += size as u64;
        vec![0u8; size]
    }
    /// Return a buffer to the pool.
    pub fn release(&mut self, mut buf: Vec<u8>) {
        buf.clear();
        let size = buf.capacity();
        // Find or create pool for this size
        if let Some(idx) = self.pools.iter().position(|(s, _)| *s == size) {
            self.pools[idx].1.push(buf);
        } else {
            self.pools.push((size, vec![buf]));
        }
    }
    pub fn total_allocated(&self) -> u64 {
        self.total_allocated_bytes + self.pools.iter().map(|(s, bufs)| *s as u64 * bufs.len() as u64).sum::<u64>()
    }
    pub fn available_count(&self) -> usize {
        self.pools.iter().map(|(_, bufs)| bufs.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn object_pool_acquire_release() {
        let mut pool = ObjectPool::<String>::new();
        let obj1 = pool.acquire();
        assert_eq!(pool.in_use_count(), 1);
        pool.release(obj1);
        assert_eq!(pool.in_use_count(), 0);
        assert_eq!(pool.available_count(), 1);
    }
    #[test] fn object_pool_reuses() {
        let mut pool = ObjectPool::<i32>::new();
        let obj = pool.acquire();
        pool.release(obj);
        let _obj2 = pool.acquire();
        assert_eq!(pool.total_allocated, 1); // only one allocation
    }
    #[test] fn buffer_pool_acquire_release() {
        let mut pool = BufferPool::new();
        let buf = pool.acquire(100);
        assert!(buf.len() >= 100);
        pool.release(buf);
        assert_eq!(pool.available_count(), 1);
    }
    #[test] fn buffer_pool_reuses() {
        let mut pool = BufferPool::new();
        let buf = pool.acquire(64);
        pool.release(buf);
        let buf2 = pool.acquire(64);
        assert_eq!(buf2.capacity(), 64);
    }
}
