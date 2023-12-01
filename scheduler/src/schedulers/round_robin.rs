use std::collections::VecDeque;
use std::num::NonZeroUsize;
use crate::{Pid, Process, ProcessState, Scheduler, StopReason, SyscallResult};
use crate::ProcessState::{Ready, Running, Waiting};
use crate::SchedulingDecision::{Deadlock, Done, Panic, Run, Sleep};
use crate::Syscall;
use crate::SyscallResult::{NoRunningProcess, Success};

#[derive(Copy, Clone, PartialEq)]
struct PCB {
    pid: usize,
    state: ProcessState,
    timings: (usize, usize, usize),
    priority: i8,
    sleep: i32,
}

impl PCB {
    fn new(pid: usize, state: ProcessState, timings: (usize, usize, usize), priority: i8) -> Self {
        PCB {
            pid,
            state,
            timings,
            priority,
            sleep: 0,
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
        self.priority
    }

    fn extra(&self) -> String {
        String::from("")
    }
}

pub struct RoundRobin {
    ready_queue: VecDeque<PCB>,
    waiting_queue: Vec<PCB>,
    current_process: Option<PCB>,
    next_pid: usize,
    timeslice: NonZeroUsize,
    minimum_remaining_timeslice: usize,
    panic: bool,
    remaining: usize,
    sleep: i32,
}

impl RoundRobin {
    pub fn new(timeslice: NonZeroUsize, minimum_remaining_timeslice: usize) -> Self {
        RoundRobin {
            ready_queue: VecDeque::new(),
            waiting_queue: Vec::new(),
            current_process: None,
            next_pid: 1,
            timeslice,
            minimum_remaining_timeslice,
            panic: false,
            remaining: timeslice.get(),
            sleep: 0,
        }
    }

    pub fn wake(&mut self) {
        self.waiting_queue.retain(|process| {
            if let Waiting {event: Some(_)} = process.state {
                true
            }
            else if process.sleep <= 0 {
                let mut ready_process = process.clone();
                ready_process.state = Ready;
                self.ready_queue.push_back(ready_process.clone());
                false
            }
            else {
                true
            }
        });
    }

    fn update_ready_timings(&mut self, remaining: usize) {
        for waiting_process in &mut self.ready_queue {
            waiting_process.timings.0 += self.remaining - remaining;
        }
    }

    fn update_waiting_timings(&mut self, remaining: usize) {
        for waiting_process in &mut self.waiting_queue {
            waiting_process.timings.0 += self.remaining - remaining;
            if let Waiting { event: Some(_) } = waiting_process.state {
                continue;
            }
            waiting_process.sleep -= (self.remaining - remaining) as i32;
        }
    }

    fn reschedule_process(&mut self, remaining: usize, process: PCB) {
        if remaining >= self.minimum_remaining_timeslice {
            self.ready_queue.push_front(process.clone());
            self.remaining = remaining;
        } else {
            self.ready_queue.push_back(process.clone());
            self.remaining = self.timeslice.get();
        }
    }
}

impl Scheduler for RoundRobin {
    fn next(&mut self) -> crate::SchedulingDecision {
        if self.panic {
            return Panic;
        }

        self.waiting_queue.sort_by_key(|process| process.sleep);

        if self.sleep != 0 {
            let amount = self.sleep;
            self.sleep = 0;
            for process in self.waiting_queue.iter_mut() {
                process.timings.0 += amount as usize;
                if let Waiting {event: Some(_)} = process.state {
                    continue;
                }
                process.sleep -= amount;
            }
        }

        self.wake();

        if self.current_process == None && self.ready_queue.is_empty() && !self.waiting_queue.is_empty() {
            let mut amount = 0;
            for process in &self.waiting_queue {
                if let Waiting {event: Some(_)} = process.state {
                    continue;
                }
                amount = process.sleep;
                break;
            }
            if amount == 0 {
                return Deadlock;
            }
            self.sleep = amount;

            return Sleep(NonZeroUsize::new(amount as usize).unwrap());
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

    fn stop(&mut self, reason: StopReason) -> SyscallResult {
        return match reason {
            StopReason::Syscall { syscall, remaining } => {
                if self.current_process == None && self.next_pid != 1 {
                    return NoRunningProcess;
                }

                match syscall {
                    Syscall::Fork(priority) => {
                        let process = PCB::new(self.next_pid, Ready, (0, 0, 0), priority);
                        self.next_pid += 1;

                        self.update_ready_timings(remaining);

                        self.update_waiting_timings(remaining);

                        self.wake();

                        self.ready_queue.push_back(process.clone());
                        if let Some(mut current_process) = self.current_process {
                            self.current_process = None;
                            current_process.state = Ready;
                            current_process.timings.2 += self.remaining - remaining - 1;
                            current_process.timings.1 += 1;
                            current_process.timings.0 += self.remaining - remaining;
                            self.reschedule_process(remaining, current_process);
                        }
                        SyscallResult::Pid(process.pid().clone())
                    }
                    Syscall::Sleep(amount) => {
                        let mut process = self.current_process.unwrap();
                        self.current_process = None;

                        self.update_ready_timings(remaining);

                        self.update_waiting_timings(remaining);

                        self.wake();

                        let event = None;
                        process.state = Waiting { event };
                        process.sleep = amount as i32;
                        process.timings.2 += self.remaining - remaining - 1;
                        process.timings.1 += 1;
                        process.timings.0 += self.remaining - remaining;

                        self.waiting_queue.push(process.clone());

                        self.remaining = self.timeslice.get();

                        Success
                    }
                    Syscall::Wait(event) => {
                        let mut process = self.current_process.unwrap();
                        self.current_process = None;

                        self.update_ready_timings(remaining);

                        self.update_waiting_timings(remaining);

                        self.wake();

                        process.state = Waiting { event: Some(event) };
                        process.timings.2 += self.remaining - remaining - 1;
                        process.timings.1 += 1;
                        process.timings.0 += self.remaining - remaining;

                        self.waiting_queue.push(process.clone());

                        self.remaining = self.timeslice.get();

                        Success
                    }
                    Syscall::Signal(signal) => {
                        let mut process = self.current_process.unwrap();
                        self.current_process = None;

                        self.update_ready_timings(remaining);

                        self.update_waiting_timings(remaining);

                        self.waiting_queue.retain(|process| {
                            if let Waiting { event: Some(event) } = process.state {
                                if event == signal {
                                    let mut ready_process = process.clone();
                                    ready_process.state = Ready;
                                    self.ready_queue.push_back(ready_process.clone());
                                    false
                                } else {
                                    true
                                }
                            } else {
                                true
                            }
                        });

                        self.wake();

                        process.state = Ready;
                        process.timings.2 += self.remaining - remaining - 1;
                        process.timings.1 += 1;
                        process.timings.0 += self.remaining - remaining;

                        self.reschedule_process(remaining, process);

                        Success
                    }
                    Syscall::Exit => {
                        let process = self.current_process.unwrap();
                        if process.pid == 1 && (!self.ready_queue.is_empty() || !self.waiting_queue.is_empty()) {
                            self.panic = true;
                        }
                        self.current_process = None;

                        self.update_ready_timings(remaining);

                        self.update_waiting_timings(remaining);

                        self.wake();

                        self.remaining = self.timeslice.get();

                        Success
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

                for waiting_process in &mut self.waiting_queue {
                    waiting_process.timings.0 += self.remaining;
                    if let Waiting { event: Some(_) } = waiting_process.state {
                        continue;
                    }
                    waiting_process.sleep -= self.remaining as i32;
                }

                self.wake();

                self.remaining = self.timeslice.get();
                self.ready_queue.push_back(process.clone());
                self.current_process = None;
                Success
            }
        }
    }

    fn list(&mut self) -> Vec<&dyn Process> {
        let mut vec: Vec<&dyn Process> = Vec::new();
        if let Some(ref process) = self.current_process {
            vec.push(process);
        }
        for process in &self.ready_queue {
            vec.push(process)
        }
        for process in &self.waiting_queue {
            vec.push(process);
        }
        vec
    }
}
