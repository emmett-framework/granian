#[cfg(not(Py_GIL_DISABLED))]
macro_rules! py_allow_threads {
    ($py:expr, $func:tt) => {
        $py.allow_threads(|| $func)
    };
}

#[cfg(Py_GIL_DISABLED)]
macro_rules! py_allow_threads {
    ($py:expr, $func:tt) => {
        $func
    };
}

pub(super) use py_allow_threads;
