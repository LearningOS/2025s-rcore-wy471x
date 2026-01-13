//! Process management syscalls
use crate::task::{change_program_brk, current_user_token, exit_current_and_run_next, suspend_current_and_run_next};
use crate::mm::{translated_byte_buffer, PageTable, VirtAddr, PTEFlags};
use crate::timer;
use core::mem::size_of;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    if _ts.is_null() {
        return -1;
    }
    let len = size_of::<TimeVal>();
    let start = _ts as usize;
    // check each page is mapped and writable by user
    let token = current_user_token();
    let page_table = PageTable::from_token(token);
    let mut addr = start;
    let end = start + len;
    while addr < end {
        let va = VirtAddr::from(addr);
        let vpn = va.floor();
        if let Some(pte) = page_table.translate(vpn) {
            let flags = pte.flags();
            if !pte.writable() || (flags & PTEFlags::U) == PTEFlags::empty() {
                return -1;
            }
        } else {
            return -1;
        }
        // advance to next page
        let next_page_va: usize = ((vpn.0 + 1) << crate::config::PAGE_SIZE_BITS).into();
        addr = core::cmp::min(next_page_va, end);
    }
    // prepare timeval bytes
    let us = timer::get_time_us();
    let tv = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let src = unsafe { core::slice::from_raw_parts((&tv as *const TimeVal) as *const u8, len) };
    let mut copied: usize = 0;
    let buffers = translated_byte_buffer(token, _ts as *const u8, len);
    for b in buffers {
        let n = core::cmp::min(b.len(), src.len() - copied);
        b[..n].copy_from_slice(&src[copied..copied + n]);
        copied += n;
        if copied >= src.len() {
            break;
        }
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    // trace_request: 0 read, 1 write, 2 syscall-count
    match _trace_request {
        2 => {
            return crate::syscall::get_syscall_count(_id) as isize;
        }
        0 | 1 => {
            // memory access at address _id
            let addr = _id;
            let token = current_user_token();
            let page_table = PageTable::from_token(token);
            let va = VirtAddr::from(addr);
            let vpn = va.floor();
            if let Some(pte) = page_table.translate(vpn) {
                let flags = pte.flags();
                // not user visible?
                if (flags & PTEFlags::U) == PTEFlags::empty() {
                    return -1;
                }
                let offset = va.page_offset();
                let ppn = pte.ppn();
                let page = ppn.get_bytes_array();
                if _trace_request == 0 {
                    // read
                    if !pte.readable() {
                        return -1;
                    }
                    let v = page[offset];
                    return v as isize;
                } else {
                    // write
                    if !pte.writable() {
                        return -1;
                    }
                    page[offset] = _data as u8;
                    return 0;
                }
            } else {
                return -1;
            }
        }
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
