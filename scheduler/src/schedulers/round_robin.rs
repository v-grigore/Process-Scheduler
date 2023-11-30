use std::collections::VecDeque;
use std::num::NonZeroUsize;
use crate::{Pid, Process, ProcessState, Scheduler, StopReason, SyscallResult};
use crate::ProcessState::{Ready, Running};
use crate::SchedulingDecision::{Done, Panic, Run};
use crate::Syscall;
use crate::SyscallResult::{NoRunningProcess, Success};

#[derive(Copy, Clone)]
struct PCB {
    pid: usize,
    state: ProcessState,
    timings: (usize, usize, usize),
}

impl PCB {
    fn new(pid: usize, state: ProcessState, timings: (usize, usize, usize)) -> Self {
        PCB {
            pid,
            state,
            timings,
        }
    }
}

impl Process for PCB {
    fn pid(&self) -> Pid {
        Pid::new(self.pid)
    }

    fn state(&self) -> ProcessState {
        self.state
    }

    fn timings(&self) -> (usize, usize, usize) {
        self.timings
    }

    fn priority(&self) -> i8 {
        0
    }

    fn extra(&self) -> String {
        String::from("")
    }
}

// impl AsRef<PCB> for PCB {
//     fn as_ref(&self) -> &PCB {
//         self
//     }
// }

pub struct RoundRobin {
    ready_queue: VecDeque<PCB>,
    waiting_queue: VecDeque<PCB>,
    current_process: Option<PCB>,
    next_pid: usize,
    timeslice: NonZeroUsize,
    minimum_remaining_timeslice: usize,
    panic: bool,
    remaining: usize,
}

impl RoundRobin {
    pub fn new(timeslice: NonZeroUsize, minimum_remaining_timeslice: usize) -> Self {
        RoundRobin {
            ready_queue: VecDeque::new(),
            waiting_queue: VecDeque::new(),
            current_process: None,
            next_pid: 1,
            timeslice,
            minimum_remaining_timeslice,
            panic: false,
            remaining: timeslice.get(),
        }
    }
}

impl Scheduler for RoundRobin {
    fn next(&mut self) -> crate::SchedulingDecision {
        if self.panic {
            return Panic;
        }

        if !self.ready_queue.is_empty() {
            let mut process = self.ready_queue.pop_front().unwrap();
            process.state = Running;
            self.current_process = Some(process.clone());
            let pid = process.pid();
            let timeslice = NonZeroUsize::new(self.remaining).unwrap();
            return Run {pid, timeslice};
        }

        if let Some(process) = self.current_process {
            let pid = process.pid();
            let timeslice = NonZeroUsize::new(self.remaining).unwrap();
            return Run {pid, timeslice};
        }

        Done
    }

    fn stop(&mut self, reason: crate::StopReason) -> crate::SyscallResult {
        match reason {
            StopReason::Syscall {syscall, remaining} => {
                match syscall {
                    Syscall::Fork(_) => {
                        let process = PCB::new(self.next_pid, ProcessState::Ready, (0, 0, 0));
                        self.next_pid += 1;

                        for waiting_process in &mut self.ready_queue {
                            waiting_process.timings.0 += self.timeslice.get() - remaining;
                        }

                        self.ready_queue.push_back(process.clone());
                        if let Some(mut current_process) = self.current_process {
                            self.current_process = None;
                            current_process.state = Ready;
                            current_process.timings.2 += self.timeslice.get() - remaining - 1;
                            current_process.timings.1 += 1;
                            current_process.timings.0 += self.timeslice.get() - remaining;
                            if remaining >= self.minimum_remaining_timeslice {
                                self.ready_queue.push_front(current_process.clone());
                                self.remaining = remaining;
                            }
                            else {
                                self.ready_queue.push_back(current_process.clone());
                            }
                        }
                        return SyscallResult::Pid(process.pid().clone());
                    }
                    Syscall::Sleep(_) => {}
                    Syscall::Wait(_) => {}
                    Syscall::Signal(_) => {}
                    Syscall::Exit => {
                        let mut process = self.current_process.unwrap();
                        if process.pid == 1 && !self.ready_queue.is_empty() && !self.waiting_queue.is_empty() {
                            self.panic = true;
                        }
                        self.current_process = None;

                        for waiting_process in &mut self.ready_queue {
                            waiting_process.timings.0 += self.timeslice.get() - remaining;
                        }

                        return Success;
                    }
                }
            }
            StopReason::Expired => {
                let mut process = self.current_process.unwrap();
                process.state = Ready;
                process.timings.2 += self.remaining;
                process.timings.0 += self.remaining;

                for waiting_process in &mut self.ready_queue {
                    waiting_process.timings.0 += self.remaining;
                }

                self.remaining = self.timeslice.get();
                self.ready_queue.push_back(process.clone());
                self.current_process = None;
                return Success;
            }
        }

        Success
    }

    fn list(&mut self) -> Vec<&dyn crate::Process> {
        let mut vec: Vec<&dyn crate::Process> = Vec::new();
        if let Some(ref process) = self.current_process {
            vec.push(process);
        }
        for process in &self.ready_queue {
            vec.push(process)
        }
        vec
    }
}
