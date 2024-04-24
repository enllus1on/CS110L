use std::collections::HashMap;
use crate::debugger_command::DebuggerCommand;
use crate::inferior::{Inferior, Status};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use rustyline::history::FileHistory;
use crate::dwarf_data::{DwarfData, Error as DwarfError};

pub struct BreakPoint {
    pub addr: usize,
    pub origin_byte: u8,
}

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<(), FileHistory>,
    inferior: Option<Inferior>,
    debug_data: Option<DwarfData>,
    breakpoints: HashMap<usize, BreakPoint>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // TODO (milestone 3): initialize the DwarfData

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<(), FileHistory>::new()
            .expect("failed to create readline");
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);
        // init debug info
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => Some(val),
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };

        // for test
        debug_data.as_ref().unwrap().print();

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data,
            breakpoints: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    self.kill();

                    if let Some(inferior) = Inferior::new(&self.target, &args) {
                        // update breakpoints 
                        self.breakpoints
                            .values_mut()
                            .filter(|bp| bp.origin_byte == 0)
                            .for_each(|bp| {
                                if let Ok(origin_byte) = inferior.write_byte(bp.addr, 0xcc) {
                                    bp.origin_byte = origin_byte;
                                }
                                else {
                                    println!("write byte: {:#x} error", 0xcc)
                                }
                            });

                        self.inferior = Some(inferior);
                        self.wakeup_wait();
                    } 
                    else {
                        println!("Error starting subprocess");
                    }
                },
                DebuggerCommand::Continue => {
                    if self.inferior.is_none() {
                        println!("no child start");
                    }
                    else {
                        self.wakeup_wait();
                    }
                },
                DebuggerCommand::Backtrace => {
                    if self.inferior.is_none() {
                        println!("no child start");
                        continue;
                    }

                    match self.inferior.as_ref().unwrap().backtrace(&self.debug_data) {
                        Ok(_) => continue,
                        Err(_) => println!("failed to backtrace")
                    }
                },
                DebuggerCommand::Break(arg) => {
                    if arg.starts_with("*") {
                        if let Some(addr) = parse_addr(&arg[1..]) {
                            self.set_bp(addr);
                        } 
                        else {
                            println!("parse addr error");
                        }
                    }
                    else {
                        if let Some(dbg_data) = self.debug_data.as_ref() {
                            let source_file = self.target.to_owned() + ".c";
                            if let Ok(line) =  arg.parse::<usize>() {
                                if let Some(addr) = dbg_data
                                .get_addr_for_line(Some(&source_file), line) {
                                    self.set_bp(addr);
                                }
                                else {
                                    println!("get addr for line error");
                                }
                            }
                            else {
                                if let Some(addr) = dbg_data
                                .get_addr_for_function(Some(&source_file), &arg) {
                                    self.set_bp(addr);
                                }
                                else {
                                    println!("get addr for function");
                                }
                            }
                        }
                    }
                },
                DebuggerCommand::Quit => {
                    self.kill();
                    return;
                }
            }
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    let _ = self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }

    fn wakeup_wait(&self) {
        match self.inferior.as_ref().unwrap().wakeup_wait(&self.breakpoints) {
            Ok(status) => {
                match status {
                    Status::Exited(ecode) => {
                        println!("child exited (status {})", ecode);
                    },
                    Status::Signaled(signal) => {
                        println!("child signaled (sigcode: {:?})", signal);
                    },
                    Status::Stopped(signal, rip) => {
                        println!("child stopped (signal: {:?})", signal);
                        // print debug info 
                        let debug_ref = self.debug_data.as_ref().unwrap();
                        let line = debug_ref
                        .get_line_from_addr(rip)
                        .expect("failed to get line info");
            
                        let func = debug_ref
                        .get_function_from_addr(rip)
                        .expect("failed to get func info");
            
                        println!("{} ({}:{})", func, line.file, line.number);
                    }
                }
            },
            Err(_) => {
                panic!("Error wakeup subprocess");
            }
        }
    }

    fn kill(&mut self) {
        if self.inferior.is_some() {
            match self.inferior.as_mut().unwrap().kill() {
                Ok(()) => println!("killed inferior (pid: {})", self.inferior.as_ref().unwrap().pid()),
                Err(_) => println!("failed to kill child")
            }
        }
    }

    fn set_bp(&mut self, addr: usize) {
        if self.inferior.is_some() {
            if self.breakpoints.contains_key(&addr) {
                println!("{:#x} has already been set", addr);
            }
            else {
                if let Ok(origin_byte) = self.inferior.as_ref().unwrap()
                    .write_byte(addr, 0xcc) {
                    let bp = BreakPoint { addr, origin_byte };
                    self.breakpoints.insert(addr, bp);
                }
                else {
                    println!("set breakpoint at: {:#x} error", addr);
                }
            }
        }
        else {
            self.breakpoints.entry(addr)
            .or_insert(
                BreakPoint {
                    addr,
                    origin_byte: 0
                }
            );
        }

        println!("set breakpoint {} at {:#x}", self.breakpoints.len() - 1, addr);
    }
}

fn parse_addr(addr: &str) -> Option<usize> {
    let addr = if addr.to_lowercase().starts_with("0x") {
        &addr[2..]
    }
    else {
        &addr
    };

    usize::from_str_radix(addr, 16).ok()
}