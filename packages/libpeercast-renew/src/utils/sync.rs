use std::sync::{MutexGuard, PoisonError, RwLockReadGuard, RwLockWriteGuard};

/// Clarify panic reason when Mutex's lock is poisoned.
/// SEE: <https://users.rust-lang.org/t/mutex-lock-error-checking/110249/>
/// # Examples
///
/// ```
/// # use std::sync::Mutex;
/// use libpeercast_re::util::mutex_poisoned;
/// # let mut mutex_val = Mutex::new(1);
/// let guard = mutex_val.lock().unwrap_or_else(mutex_poisoned);
/// assert_eq!(*guard, 1);
/// ```
pub fn mutex_poisoned<'a, T>(_: PoisonError<MutexGuard<T>>) -> MutexGuard<'a, T> {
    panic!("mutex poisoned")
}

/// Clarify panic reason when RwLock.read() is poisoned.
pub fn rwlock_read_poisoned<'a, T>(_: PoisonError<RwLockReadGuard<T>>) -> RwLockReadGuard<'a, T> {
    panic!("RwLock read poisoned")
}

/// Clarify panic reason when RwLock.read() is poisoned.
pub fn rwlock_write_poisoned<'a, T>(_: PoisonError<RwLockWriteGuard<T>>) -> RwLockWriteGuard<'a, T> {
    panic!("RwLock write poisoned")
}
