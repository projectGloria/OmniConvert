//! Win32 Job Object wrapper — the actual orphan-process guarantee (PLAN D3).
//!
//! Every sidecar job gets its own Job Object flagged KILL_ON_JOB_CLOSE:
//! - Cancel → `TerminateJobObject` kills the whole tree. That matters for pandoc,
//!   which spawns `typst` as a grandchild; killing pandoc's PID alone would orphan it.
//! - Job finishes → handle dropped → anything still alive in the tree dies.
//! - App crashes / killed from Task Manager → the OS closes our handles → same.

use std::io;
use std::os::windows::io::RawHandle;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

pub struct JobObject(HANDLE);

// SAFETY: a Job Object HANDLE is a kernel handle usable from any thread;
// we never alias it mutably and close it exactly once in Drop.
unsafe impl Send for JobObject {}
unsafe impl Sync for JobObject {}

impl JobObject {
    pub fn new() -> io::Result<Self> {
        // SAFETY: null security attributes + null name = anonymous, non-inheritable job.
        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = Self(handle);

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: pointer + size describe a valid, fully initialized struct of the declared class.
        let ok = unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }

    /// Children the process spawns *after* this call inherit the job automatically.
    pub fn assign(&self, process: RawHandle) -> io::Result<()> {
        // SAFETY: `process` is a live process handle owned by tokio's Child for the call's duration.
        if unsafe { AssignProcessToJobObject(self.0, process as HANDLE) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Kill every process in the tree. Exit code 1 so it reads as a failure, not success.
    pub fn terminate(&self) {
        // SAFETY: valid job handle; failure (already empty) is harmless.
        unsafe { TerminateJobObject(self.0, 1) };
    }
}

impl Drop for JobObject {
    fn drop(&mut self) {
        // SAFETY: we own the handle; closing it triggers KILL_ON_JOB_CLOSE.
        unsafe { CloseHandle(self.0) };
    }
}
