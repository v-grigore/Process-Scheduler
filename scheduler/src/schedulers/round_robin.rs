use std::collections::VecDeque;
use std::num::NonZeroUsize;
use crate::{Pid, Process, ProcessState, Scheduler, StopReason, SyscallResult};
use crate::SchedulingDecision::Done;
use crate::Syscall;
use crate::SyscallResult::Success;

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
        String::new()
    }
}

pub struct RoundRobin {
    ready_queue: VecDeque<PCB>,
    waiting_queue: VecDeque<PCB>,
    current_process: Option<PCB>,
    next_pid: usize,
    timeslice: NonZeroUsize,
    minimum_remaining_timeslice: usize,
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
        }
    }
}

impl Scheduler for RoundRobin {
    fn next(&mut self) -> crate::SchedulingDecision {
        Done
    }

    fn stop(&mut self, reason: crate::StopReason) -> crate::SyscallResult {
        match reason {
            StopReason::Syscall {syscall, remaining} => {
                match syscall {
                    Syscall::Fork(_) => {
                        if self.next_pid == 1 {
                            let process = PCB::new(1, ProcessState::Running, (0, 0, 0));
                            self.current_process = Some(process);
                            self.next_pid += 1;
                            return SyscallResult::Pid(process.pid().clone());
                        }
                    }
                    Syscall::Sleep(_) => {}
                    Syscall::Wait(_) => {}
                    Syscall::Signal(_) => {}
                    Syscall::Exit => {}
                }
            }
            StopReason::Expired => {

            }
        }

        Success
    }

    fn list(&mut self) -> Vec<&dyn crate::Process> {
        let mut vec: Vec<&dyn crate::Process> = Vec::new();
        if let Some(ref process) = self.current_process {
            vec.push(process);
        }
        vec
    }
}
