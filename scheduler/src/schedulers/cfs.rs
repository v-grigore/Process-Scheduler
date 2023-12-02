use std::cmp::Ordering;
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
    vruntime: usize,
}

impl PCB {
    fn new(pid: usize, state: ProcessState, timings: (usize, usize, usize), priority: i8) -> Self {
        PCB {
            pid,
            state,
            timings,
            priority,
            sleep: 0,
            vruntime: 0,
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
        format!("vruntime={}", self.vruntime)
    }
}

impl PartialOrd for PCB {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.vruntime == other.vruntime {
            Some(self.pid.cmp(&other.pid))
        }
        else {
            Some(self.vruntime.cmp(&other.vruntime))
        }
    }
}

pub struct CFS {
    ready_queue: VecDeque<PCB>,
    waiting_queue: Vec<PCB>,
    current_process: Option<PCB>,
    next_pid: usize,
    timeslice: NonZeroUsize,
    minimum_remaining_timeslice: usize,
    panic: bool,
    remaining: usize,
    sleep: i32,
    cpu_time: NonZeroUsize,
    minimum_vruntime: usize,
}

impl CFS {
    pub fn new(cpu_time: NonZeroUsize, minimum_remaining_timeslice: usize) -> Self {
        CFS {
            ready_queue: VecDeque::new(),
            waiting_queue: Vec::new(),
            current_process: None,
            next_pid: 1,
            timeslice: cpu_time,
            minimum_remaining_timeslice,
            panic: false,
            remaining: cpu_time.get(),
            sleep: 0,
            cpu_time,
            minimum_vruntime: 0,
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
            self.ready_queue.make_contiguous().sort_by(|a, b| a.partial_cmp(b).unwrap());
            self.ready_queue.push_front(process.clone());
            self.remaining = remaining;
        } else {
            self.ready_queue.push_back(process.clone());
            self.ready_queue.make_contiguous().sort_by(|a, b| a.partial_cmp(b).unwrap());
            self.remaining = self.timeslice.get();
        }
    }

    fn update_minimum_vruntime(&mut self, current: usize) {
        let mut all_vruntime: Vec<usize> = self.ready_queue.iter().map(|process| process.vruntime)
            .chain(self.waiting_queue.iter().map(|process| process.vruntime))
            .collect();

        all_vruntime.push(current);
        
        if let Some(min) = all_vruntime.iter().cloned().min() {
            self.minimum_vruntime = min;
        }
    }
}

impl Scheduler for CFS {
    fn next(&mut self) -> crate::SchedulingDecision {
        if self.panic {
            return Panic;
        }

        self.waiting_queue.sort_by_key(|process| process.sleep);

        if self.sleep != 0 {
            self.ready_queue.make_contiguous().sort_by(|a, b| a.partial_cmp(b).unwrap());

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
                        let mut process = PCB::new(self.next_pid, Ready, (0, 0, 0), priority);
                        self.next_pid += 1;

                        self.update_ready_timings(remaining);

                        self.update_waiting_timings(remaining);

                        self.wake();

                        if process.pid == 1 {
                            self.ready_queue.push_back(process.clone());
                        }

                        if let Some(mut current_process) = self.current_process {
                            self.current_process = None;
                            current_process.state = Ready;
                            current_process.timings.2 += self.remaining - remaining - 1;
                            current_process.timings.1 += 1;
                            current_process.timings.0 += self.remaining - remaining;
                            current_process.vruntime += self.remaining - remaining;

                            self.update_minimum_vruntime(current_process.vruntime);
                            process.vruntime = self.minimum_vruntime;
                            self.ready_queue.push_back(process.clone());

                            self.timeslice = NonZeroUsize::new(self.cpu_time.get() / (self.ready_queue.len() + 1)).unwrap();

                            self.reschedule_process(self.timeslice.get().min(remaining), current_process);
                        }
                        SyscallResult::Pid(process.pid().clone())
                    }
                    Syscall::Sleep(amount) => {
                        let mut process = self.current_process.unwrap();
                        self.current_process = None;

                        self.update_ready_timings(remaining);

                        self.update_waiting_timings(remaining);

                        self.wake();

                        self.timeslice = NonZeroUsize::new(self.cpu_time.get() / self.ready_queue.len()).unwrap();

                        let event = None;
                        process.state = Waiting { event };
                        process.sleep = amount as i32;
                        process.timings.2 += self.remaining - remaining - 1;
                        process.timings.1 += 1;
                        process.timings.0 += self.remaining - remaining;
                        process.vruntime += self.remaining - remaining;

                        self.waiting_queue.push(process.clone());

                        self.remaining = self.timeslice.get();

                        self.ready_queue.make_contiguous().sort_by(|a, b| a.partial_cmp(b).unwrap());

                        Success
                    }
                    Syscall::Wait(event) => {
                        let mut process = self.current_process.unwrap();
                        self.current_process = None;

                        self.update_ready_timings(remaining);

                        self.update_waiting_timings(remaining);

                        self.wake();

                        self.timeslice = NonZeroUsize::new(self.cpu_time.get() / self.ready_queue.len()).unwrap();

                        process.state = Waiting { event: Some(event) };
                        process.timings.2 += self.remaining - remaining - 1;
                        process.timings.1 += 1;
                        process.timings.0 += self.remaining - remaining;
                        process.vruntime += self.remaining - remaining;

                        self.waiting_queue.push(process.clone());

                        self.remaining = self.timeslice.get();

                        self.ready_queue.make_contiguous().sort_by(|a, b| a.partial_cmp(b).unwrap());

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

                        self.timeslice = NonZeroUsize::new(self.cpu_time.get() / (self.ready_queue.len() + 1)).unwrap();

                        process.state = Ready;
                        process.timings.2 += self.remaining - remaining - 1;
                        process.timings.1 += 1;
                        process.timings.0 += self.remaining - remaining;
                        process.vruntime += self.remaining - remaining;

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

                        if process.pid != 1 {
                            self.timeslice = NonZeroUsize::new(self.cpu_time.get() / self.ready_queue.len()).unwrap();
                        }

                        self.remaining = self.timeslice.get();

                        self.ready_queue.make_contiguous().sort_by(|a, b| a.partial_cmp(b).unwrap());

                        Success
                    }
                }
            }
            StopReason::Expired => {
                let mut process = self.current_process.unwrap();
                process.state = Ready;
                process.timings.2 += self.remaining;
                process.timings.0 += self.remaining;
                process.vruntime += self.remaining;

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

                self.timeslice = NonZeroUsize::new(self.cpu_time.get() / (self.ready_queue.len() + 1)).unwrap();

                self.remaining = self.timeslice.get();
                self.ready_queue.push_back(process.clone());
                self.current_process = None;

                self.ready_queue.make_contiguous().sort_by(|a, b| a.partial_cmp(b).unwrap());

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
