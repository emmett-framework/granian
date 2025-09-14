use pyo3::{
    exceptions::PyRuntimeError,
    prelude::*,
    types::{IntoPyDict, PyDict},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

#[pyclass(frozen, module = "granian._granian")]
struct ProcInfoCollector {
    sys: Arc<Mutex<System>>,
}

#[pymethods]
impl ProcInfoCollector {
    #[new]
    fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_memory()),
        );
        Self {
            sys: Arc::new(Mutex::new(sys)),
        }
    }

    #[pyo3(signature = (pids = None))]
    fn memory<'p>(&self, py: Python<'p>, pids: Option<Vec<u32>>) -> PyResult<Bound<'p, PyDict>> {
        let pids = pids.map_or_else(
            || vec![sysinfo::get_current_pid().unwrap()],
            |v| v.into_iter().map(sysinfo::Pid::from_u32).collect(),
        );
        let ret = py.detach(|| {
            let mut sys = self.sys.lock().unwrap();
            sys.refresh_processes(ProcessesToUpdate::Some(&pids), false);
            let mut ret = HashMap::with_capacity(pids.len());
            for pid in pids {
                let proc = sys
                    .process(pid)
                    .ok_or(PyRuntimeError::new_err("unable to refresh process"))?;
                ret.insert(pid.as_u32(), proc.memory());
            }
            Ok::<HashMap<u32, u64>, PyErr>(ret)
        })?;
        ret.into_py_dict(py)
    }
}

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    sysinfo::set_open_files_limit(0);
    module.add_class::<ProcInfoCollector>()?;

    Ok(())
}
