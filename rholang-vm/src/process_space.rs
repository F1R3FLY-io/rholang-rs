use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::process::Process;

/// Abstract storage for Process objects addressed by a string path.
/// The path format is up to the caller (e.g. "/process/{pid}").
pub trait ProcessSpace: Send + Sync {
    fn put(&self, path: &str, process: Arc<Process>);
    fn get(&self, path: &str) -> Option<Arc<Process>>;
    fn remove(&self, path: &str) -> Option<Arc<Process>>;
    fn clear(&self);
}

/// Simple in-memory HashMap-backed process space.
pub struct InMemoryProcessSpace {
    inner: RwLock<HashMap<String, Arc<Process>>>,
}

impl InMemoryProcessSpace {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryProcessSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessSpace for InMemoryProcessSpace {
    fn put(&self, path: &str, process: Arc<Process>) {
        let mut map = self.inner.write().expect("poisoned");
        map.insert(path.to_string(), process);
    }

    fn get(&self, path: &str) -> Option<Arc<Process>> {
        let map = self.inner.read().expect("poisoned");
        map.get(path).cloned()
    }

    fn remove(&self, path: &str) -> Option<Arc<Process>> {
        let mut map = self.inner.write().expect("poisoned");
        map.remove(path)
    }

    fn clear(&self) {
        let mut map = self.inner.write().expect("poisoned");
        map.clear();
    }
}

#[cfg(feature = "process-space")]
mod pathmap_impl {
    use super::ProcessSpace;
    use crate::process::Process;
    use std::sync::{Arc, RwLock};

    // Note: The `pathmap` crate API is used here behind a feature gate.
    // The exact import path/type names may differ slightly and can be adjusted when enabling the feature.
    use pathmap::PathMap;

    pub struct PathMapProcessSpace {
        inner: RwLock<PathMap<Arc<Process>>>,
    }

    impl PathMapProcessSpace {
        pub fn new() -> Self {
            Self {
                inner: RwLock::new(PathMap::new()),
            }
        }
    }

    impl Default for PathMapProcessSpace {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ProcessSpace for PathMapProcessSpace {
        fn put(&self, path: &str, process: Arc<Process>) {
            let mut pm = self.inner.write().expect("poisoned");
            // Store using a POSIX-like path (e.g., "/process/1").
            pm.insert(path, process);
        }

        fn get(&self, path: &str) -> Option<Arc<Process>> {
            let pm = self.inner.read().expect("poisoned");
            pm.get(path).cloned()
        }

        fn remove(&self, path: &str) -> Option<Arc<Process>> {
            let mut pm = self.inner.write().expect("poisoned");
            pm.remove(path)
        }

        fn clear(&self) {
            let mut pm = self.inner.write().expect("poisoned");
            // PathMap does not expose a clear API; reinitialize instead.
            *pm = pathmap::PathMap::new();
        }
    }
}

#[cfg(feature = "process-space")]
pub use pathmap_impl::PathMapProcessSpace as DefaultProcessSpace;

#[cfg(not(feature = "process-space"))]
pub type DefaultProcessSpace = InMemoryProcessSpace;

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn mkp(src: &str) -> Arc<Process> { Arc::new(Process::new(vec![], src)) }

    #[test]
    fn inmem_put_get_remove_clear() {
        let space = InMemoryProcessSpace::new();
        let p1 = mkp("p1");
        let p2 = mkp("p2");
        space.put("/process/1", p1.clone());
        space.put("/process/2", p2.clone());
        assert_eq!(space.get("/process/1").unwrap().source_ref, "p1");
        assert_eq!(space.get("/process/2").unwrap().source_ref, "p2");
        let removed = space.remove("/process/1").unwrap();
        assert_eq!(removed.source_ref, "p1");
        assert!(space.get("/process/1").is_none());
        space.clear();
        assert!(space.get("/process/2").is_none());
    }

    #[test]
    fn inmem_concurrent_put_and_get() {
        let space = InMemoryProcessSpace::new();
        let space_arc = Arc::new(space);
        let mut handles = vec![];
        for i in 0..8u32 {
            let s = space_arc.clone();
            handles.push(thread::spawn(move || {
                let path = format!("/process/{}", i);
                s.put(&path, mkp(&format!("src:{}", i)));
            }));
        }
        for h in handles { h.join().unwrap(); }
        for i in 0..8u32 {
            let path = format!("/process/{}", i);
            assert_eq!(space_arc.get(&path).unwrap().source_ref, format!("src:{}", i));
        }
    }

    #[cfg(feature = "process-space")]
    #[test]
    fn pathmap_put_get_remove_clear() {
        let space = super::pathmap_impl::PathMapProcessSpace::new();
        let p1 = mkp("p1");
        let p2 = mkp("p2");
        space.put("/process/1", p1.clone());
        space.put("/process/2", p2.clone());
        assert_eq!(space.get("/process/1").unwrap().source_ref, "p1");
        assert_eq!(space.get("/process/2").unwrap().source_ref, "p2");
        let removed = space.remove("/process/1").unwrap();
        assert_eq!(removed.source_ref, "p1");
        assert!(space.get("/process/1").is_none());
        space.clear();
        assert!(space.get("/process/2").is_none());
    }
}
