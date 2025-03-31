use std::sync::{MutexGuard, PoisonError, RwLockReadGuard, RwLockWriteGuard};

/// Clarify panic reason when Mutex's lock is poisoned.
/// SEE: <https://users.rust-lang.org/t/mutex-lock-error-checking/110249/>
/// # Examples
///
/// ```
/// # use std::sync::Mutex;
/// use peercast_re::util::mutex_poisoned;
/// # let mut mutex_val = Mutex::new(1);
/// let guard = mutex_val.lock().unwrap_or_else(mutex_poisoned);
/// assert_eq!(*guard, 1);
/// ```
pub fn mutex_poisoned<T>(_: PoisonError<MutexGuard<T>>) -> MutexGuard<'_, T> {
    panic!("mutex poisoned")
}

/// Clarify panic reason when RwLock.read() is poisoned.
pub fn rwlock_read_poisoned<T>(_: PoisonError<RwLockReadGuard<T>>) -> RwLockReadGuard<'_, T> {
    panic!("RwLock read poisoned")
}

/// Clarify panic reason when RwLock.read() is poisoned.
pub fn rwlock_write_poisoned<T>(_: PoisonError<RwLockWriteGuard<T>>) -> RwLockWriteGuard<'_, T> {
    panic!("RwLock write poisoned")
}
