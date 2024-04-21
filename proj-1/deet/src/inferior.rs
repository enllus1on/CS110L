use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::process::Child;
use std::mem::size_of;
use crate::dwarf_data::DwarfData;

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>) -> Option<Inferior> {
        // TODO: implement me!
        use std::process::Command;
        use std::os::unix::process::CommandExt;

        let mut cmd = Command::new(target);

        unsafe {
            cmd.args(args)
            .pre_exec(|| child_traceme());
        }
        
        let child = cmd.spawn().ok()?;
        let inferior = Inferior { child };

        match inferior.wait(Some(WaitPidFlag::WSTOPPED)).ok()? {
            Status::Stopped(_, _) => Some(inferior),
            _ => None,
        }

    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            },
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    pub fn wakeup_wait(&self) -> Result<Status, nix::Error> {
        ptrace::cont(self.pid(), None)?;
        
        self.wait(None)
    }

    pub fn kill(&mut self) -> Result<(), nix::Error> {
        ptrace::kill(self.pid())
    }

    pub fn backtrace(&self, debug_data: &Option<DwarfData>) -> Result<(), nix::Error> {
        let regs = ptrace::getregs(self.pid())?;
        let mut rip = regs.rip as usize;
        let mut rbp = regs.rbp as usize;
        let debug_ref = debug_data.as_ref().unwrap();


        loop {
            let line = debug_ref
            .get_line_from_addr(rip)
            .expect("failed to get line info");

            let func = debug_ref
            .get_function_from_addr(rip)
            .expect("failed to get func info");

            println!("{} ({}:{})", func, line.file, line.number);

            if func.contains("main") {
                break;
            }

            rip = ptrace::read(self.pid(), (rbp + 8) as ptrace::AddressType)? as usize;
            rbp = ptrace::read(self.pid(), rbp as ptrace::AddressType)? as usize;
        }

        Ok(())
    }

    pub fn write_byte(&self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        unsafe { 
            ptrace::write(
                    self.pid(),
                    aligned_addr as ptrace::AddressType,
                    updated_word as *mut std::ffi::c_void,
                )? 
        };
        Ok(orig_byte as u8)
    }
}

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}
