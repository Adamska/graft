use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::ast_diff::{self, Analysis};
use crate::changeset::{self, DiffTarget};
use crate::error::{Error, Result};
use crate::git::repo::{self, RepoSnapshot};
use crate::git::search::GraphIndex;

const ANALYSIS_CACHE: usize = 4;

/// Everything tied to the repository currently open in the window.
pub struct Session {
    pub repo: gix::ThreadSafeRepository,
    pub root: PathBuf,
    pub snapshot: RwLock<RepoSnapshot>,
    pub graph: RwLock<GraphIndex>,
    /// Recent changeset analyses, newest first, reused while the inspector pages through files.
    analyses: Mutex<Vec<(DiffTarget, Arc<Analysis>)>>,
    stream_epoch: AtomicU64,
}

impl Session {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let repository = repo::open(path)?;
        let snapshot = repo::snapshot(&repository)?;
        Ok(Self {
            root: repo::root_of(&repository),
            repo: repository.into_sync(),
            snapshot: RwLock::new(snapshot),
            graph: RwLock::default(),
            analyses: Mutex::default(),
            stream_epoch: AtomicU64::new(0),
        })
    }

    pub fn repository(&self) -> gix::Repository {
        self.repo.to_thread_local()
    }

    pub fn snapshot(&self) -> RepoSnapshot {
        self.snapshot.read().map(|snapshot| snapshot.clone()).unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }

    pub fn refresh_snapshot(&self) -> Result<RepoSnapshot> {
        let snapshot = repo::snapshot(&self.repository())?;
        if let Ok(mut current) = self.snapshot.write() {
            *current = snapshot.clone();
        }
        Ok(snapshot)
    }

    /// Returns the cached analysis for `target`, computing it on first use.
    pub fn analysis(&self, target: &DiffTarget) -> Result<Arc<Analysis>> {
        let cached = self.analyses.lock().ok().and_then(|cache| {
            cache.iter().find(|(cached, _)| cached == target).map(|(_, analysis)| Arc::clone(analysis))
        });
        match cached {
            Some(analysis) => Ok(analysis),
            None => self.analyze(target),
        }
    }

    /// Recomputes the analysis for `target` and caches it, evicting the oldest entry.
    pub fn analyze(&self, target: &DiffTarget) -> Result<Arc<Analysis>> {
        let sources = changeset::sources(&self.repository(), target)?;
        let analysis = Arc::new(ast_diff::analyze(sources));
        if let Ok(mut cache) = self.analyses.lock() {
            cache.retain(|(cached, _)| cached != target);
            cache.truncate(ANALYSIS_CACHE - 1);
            cache.insert(0, (target.clone(), Arc::clone(&analysis)));
        }
        Ok(analysis)
    }

    /// Runs a command that changes the repository, dropping analyses it may invalidate.
    pub fn mutate<T>(&self, action: impl FnOnce(&std::path::Path) -> Result<T>) -> Result<T> {
        self.invalidate_working();
        action(&self.root)
    }

    /// Drops analyses of the index and working tree after either has been mutated.
    pub fn invalidate_working(&self) {
        if let Ok(mut cache) = self.analyses.lock() {
            cache.retain(|(target, _)| matches!(target, DiffTarget::Commit { .. }));
        }
    }

    /// Starts a new graph stream and invalidates any stream still in flight.
    pub fn begin_stream(&self) -> u64 {
        if let Ok(mut graph) = self.graph.write() {
            *graph = GraphIndex::default();
        }
        self.stream_epoch.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn is_current_stream(&self, epoch: u64) -> bool {
        self.stream_epoch.load(Ordering::SeqCst) == epoch
    }
}

#[derive(Default)]
pub struct AppState {
    session: RwLock<Option<Arc<Session>>>,
}

impl AppState {
    pub fn session(&self) -> Result<Arc<Session>> {
        self.session.read().ok().and_then(|session| session.clone()).ok_or(Error::NoRepository)
    }

    pub fn activate(&self, session: Session) -> Arc<Session> {
        let session = Arc::new(session);
        if let Ok(mut current) = self.session.write() {
            *current = Some(Arc::clone(&session));
        }
        session
    }

    pub fn is_active(&self, session: &Arc<Session>) -> bool {
        self.session.read().is_ok_and(|current| current.as_ref().is_some_and(|current| Arc::ptr_eq(current, session)))
    }
}
