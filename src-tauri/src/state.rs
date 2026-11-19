use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::ast_diff::Analysis;
use crate::changeset::DiffTarget;
use crate::error::{Error, Result};
use crate::git::repo::{self, RepoSnapshot};
use crate::git::search::GraphIndex;

/// Everything tied to the repository currently open in the window.
pub struct Session {
    pub repo: gix::ThreadSafeRepository,
    pub root: PathBuf,
    pub snapshot: RwLock<RepoSnapshot>,
    pub graph: RwLock<GraphIndex>,
    /// Most recent changeset analysis, reused while the inspector pages through its files.
    pub analysis: RwLock<Option<Arc<(DiffTarget, Analysis)>>>,
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
            analysis: RwLock::default(),
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

    pub fn store_analysis(&self, target: DiffTarget, analysis: Analysis) -> Arc<(DiffTarget, Analysis)> {
        let entry = Arc::new((target, analysis));
        if let Ok(mut current) = self.analysis.write() {
            *current = Some(Arc::clone(&entry));
        }
        entry
    }

    pub fn analysis(&self) -> Result<Arc<(DiffTarget, Analysis)>> {
        let current = self.analysis.read().ok().and_then(|analysis| analysis.clone());
        current.ok_or_else(|| Error::invalid("no changeset has been analyzed yet"))
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
