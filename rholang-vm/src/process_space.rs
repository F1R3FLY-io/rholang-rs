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

    #[cfg(feature = "process-space")]
    #[test]
    fn pathmap_deep_paths_and_overwrite_and_missing_remove_and_bulk() {
        let space = super::pathmap_impl::PathMapProcessSpace::new();

        // Deep nested path support
        let deep_path = "/process/a/b/c";
        let p_deep = mkp("deep");
        space.put(deep_path, p_deep.clone());
        assert_eq!(space.get(deep_path).unwrap().source_ref, "deep");

        // Overwrite semantics: inserting at the same path replaces the value
        let p_over = mkp("overwritten");
        space.put(deep_path, p_over.clone());
        assert_eq!(space.get(deep_path).unwrap().source_ref, "overwritten");

        // Removing a non-existent path returns None
        assert!(space.remove("/process/does/not/exist").is_none());

        // Sibling paths do not interfere
        let p_sib1 = mkp("sib1");
        let p_sib2 = mkp("sib2");
        space.put("/process/a/x", p_sib1.clone());
        space.put("/process/a/y", p_sib2.clone());
        assert_eq!(space.get("/process/a/x").unwrap().source_ref, "sib1");
        assert_eq!(space.get("/process/a/y").unwrap().source_ref, "sib2");

        // Bulk insert and sampling
        for i in 0..1_000u32 {
            let path = format!("/process/bulk/{}", i);
            space.put(&path, mkp(&format!("bulk:{}", i)));
        }
        // spot check a few
        for i in [0u32, 1, 10, 123, 999] {
            let path = format!("/process/bulk/{}", i);
            assert_eq!(space.get(&path).unwrap().source_ref, format!("bulk:{}", i));
        }

        // Path normalization expectation: current implementation treats paths verbatim
        space.put("process/no/leading/slash", mkp("no_slash"));
        assert_eq!(space.get("process/no/leading/slash").unwrap().source_ref, "no_slash");
        // trailing slash is also a distinct key
        space.put("/process/trailing/", mkp("trail"));
        assert_eq!(space.get("/process/trailing/").unwrap().source_ref, "trail");
    }
}
