# Rust Schedulers Implementation

## Table of contents
- [Round Robin Scheduler](#round-robin-scheduler)
  - [PCB (Process Control Block) Structure](#pcb-process-control-block-structure)
  - [Round Robin Scheduler](#round-robin-scheduler-1)
  - [Queues](#queues)
  - [Current Process](#current-process)
  - [Scheduler Configuration](#scheduler-configuration)
  - [Other Fields](#other-fields)
  - [Methods](#methods)
  - [Scheduler Logic](#scheduler-logic)
- [Priority Queue Scheduler](#priority-queue-scheduler)
  - [PCB (Process Control Block) Structure](#pcb-process-control-block-structure-1)
  - [Priority Queue Scheduler](#priority-queue-scheduler-1)
  - [Queues](#queues-1)
  - [Current Process](#current-process-1)
  - [Scheduler Configuration](#scheduler-configuration-1)
  - [Other Fields](#other-fields-1)
  - [Methods](#methods-1)
  - [Scheduler Logic](#scheduler-logic-1)
- [Completely Fair Scheduler (CFS)](#completely-fair-scheduler-cfs)
  - [PCB (Process Control Block) Structure](#pcb-process-control-block-structure-2)
  - [Completely Fair Scheduler (CFS)](#completely-fair-scheduler-cfs-1)
  - [Queues](#queues-2)
  - [Current Process](#current-process-2)
  - [Scheduler Configuration](#scheduler-configuration-2)
  - [Other Fields](#other-fields-2)
  - [Methods](#methods-2)
  - [Scheduler Logic](#scheduler-logic-2)

## Round Robin Scheduler
This section explains the implementation of the Round Robin scheduler in Rust.

### PCB (Process Control Block) Structure
The `PCB` struct represents the process control block. It contains information about a process, such as its process ID (`pid`), state, timings, priority, and sleep time. The `PCB` struct implements the `Process` trait.

### Round Robin Scheduler
The `RoundRobin` struct is the main implementation of the Round Robin scheduler. It has the following key components:

#### Queues
- `ready_queue`: A `VecDeque` containing processes that are ready to be scheduled.
- `waiting_queue`: A `Vec` containing processes that are waiting for an event or sleeping.
#### Current Process
- `current_process`: An `Option<PCB>` representing the currently scheduled process.
#### Scheduler Configuration
- `timeslice`: The time quantum assigned to each process.
- `minimum_remaining_timeslice`: The minimum remaining timeslice required to reschedule a process.
#### Other Fields
- `next_pid`: A counter for assigning unique process IDs.
- `panic`: A flag indicating whether the scheduler is in a panic state.
- `remaining`: The remaining timeslice for the current process.
- `sleep`: A temporary field used for handling sleep operations.
#### Methods
- `new`: Creates a new instance of the Round Robin scheduler.
- `wake`: Handles waking up processes in the waiting queue.
- `update_ready_timings` and `update_waiting_timings`: Updates timings for processes in the ready and waiting queues, respectively.
- `reschedule_process`: Reschedules a process based on the remaining timeslice.
#### Scheduler Logic
1. **Initialization**: The scheduler is initialized with empty queues and default values.
2. **Waking Up Processes**: The `wake` method is responsible for waking up processes in the waiting queue.
3. **Updating Timings**: The `update_ready_timings` and `update_waiting_timings` methods update timings for processes in the ready and waiting queues.
4. **Process Scheduling**: The `next` method determines the next process to be scheduled based on the current state of queues.
5. **Handling Syscalls**: The `stop` method handles syscall requests, such as fork, sleep, wait, signal, and exit.
6. **Listing Processes**: The `list` method provides a list of processes in the order they are scheduled.

## Priority Queue Scheduler
This section explains the implementation of the Priority Queue scheduler in Rust.

### PCB (Process Control Block) Structure
The `PCB` struct represents the process control block. It contains information about a process, such as its process ID (`pid`), state, timings, priority, sleep time, and maximum priority. The `PCB` struct implements the `Process` trait and `PartialOrd` trait based on priority.

### Priority Queue Scheduler
The `PriorityQueue` struct is the main implementation of the Priority Queue scheduler. It has the following key components:

#### Queues
- `ready_queue`: A `VecDeque` containing processes that are ready to be scheduled.
- `waiting_queue`: A `Vec` containing processes that are waiting for an event or sleeping.
#### Current Process
- `current_process`: An `Option<PCB>` representing the currently scheduled process.
#### Scheduler Configuration
- `timeslice`: The time quantum assigned to each process.
- `minimum_remaining_timeslice`: The minimum remaining timeslice required to reschedule a process.
#### Other Fields
- `next_pid`: A counter for assigning unique process IDs.
- `panic`: A flag indicating whether the scheduler is in a panic state.
- `remaining`: The remaining timeslice for the current process.
- `sleep`: A temporary field used for handling sleep operations.
#### Methods
- `new`: Creates a new instance of the Priority Queue scheduler.
- `wake`: Handles waking up processes in the waiting queue.
- `update_ready_timings` and `update_waiting_timings`: Updates timings for processes in the ready and waiting queues, respectively.
- `reschedule_process`: Reschedules a process based on the remaining timeslice.
#### Scheduler Logic
1. **Initialization**: The scheduler is initialized with empty queues and default values.
2. **Waking Up Processes**: The `wake` method is responsible for waking up processes in the waiting queue.
3. **Updating Timings**: The `update_ready_timings` and `update_waiting_timings` methods update timings for processes in the ready and waiting queues.
4. **Process Scheduling**: The `next` method determines the next process to be scheduled based on the current state of queues, prioritizing processes with higher priority.
5. **Handling Syscalls**: The `stop` method handles syscall requests, such as fork, sleep, wait, signal, and exit.
6. **Listing Processes**: The `list` method provides a list of processes in the order they are scheduled.

## Completely Fair Scheduler (CFS)
This section explains the implementation of the Completely Fair Scheduler (CFS) in Rust.

### PCB (Process Control Block) Structure
The `PCB` struct represents the process control block. It contains information about a process, such as its process ID (`pid`), state, timings, priority, sleep time, and virtual runtime (`vruntime`). The `PCB` struct implements the `Process` trait and `PartialOrd` trait based on virtual runtime.

### Completely Fair Scheduler (CFS)
The `CFS` struct is the main implementation of the Completely Fair Scheduler. It has the following key components:

#### Queues
- `ready_queue`: A `VecDeque` containing processes that are ready to be scheduled.
- `waiting_queue`: A `Vec` containing processes that are waiting for an event or sleeping.
#### Current Process
- `current_process`: An `Option<PCB>` representing the currently scheduled process.
#### Scheduler Configuration
- `timeslice`: The time quantum assigned to each process.
- `minimum_remaining_timeslice`: The minimum remaining timeslice required to reschedule a process.
- `cpu_time`: The total CPU time allocated to the scheduler.
- `minimum_vruntime`: The minimum virtual runtime among all processes.
#### Other Fields
- `next_pid`: A counter for assigning unique process IDs.
- `panic`: A flag indicating whether the scheduler is in a panic state.
- `remaining`: The remaining timeslice for the current process.
- `sleep`: A temporary field used for handling sleep operations.
#### Methods
- `new`: Creates a new instance of the CFS scheduler.
- `wake`: Handles waking up processes in the waiting queue.
- `update_ready_timings` and `update_waiting_timings`: Updates timings for processes in the ready and waiting queues, respectively.
- `reschedule_process`: Reschedules a process based on the remaining timeslice.
- `update_minimum_vruntime`: Updates the minimum virtual runtime among all processes.
- `update_timeslice`: Updates the timeslice based on the number of processes.
#### Scheduler Logic
1. **Initialization**: The scheduler is initialized with empty queues and default values.
2. **Waking Up Processes**: The `wake` method is responsible for waking up processes in the waiting queue.
3. **Updating Timings**: The `update_ready_timings` and `update_waiting_timings` methods update timings for processes in the ready and waiting queues.
4. **Updating Minimum Virtual Runtime**: The `update_minimum_vruntime` method updates the minimum virtual runtime among all processes.
5. **Updating Timeslice**: The `update_timeslice` method updates the timeslice based on the number of processes.
6. **Process Scheduling**: The `next` method determines the next process to be scheduled based on the current state of queues, prioritizing processes with lower virtual runtime.
7. **Handling Syscalls**: The `stop` method handles syscall requests, such as fork, sleep, wait, signal, and exit.
8. **Listing Processes**: The `list` method provides a list of processes in the order they are scheduled.