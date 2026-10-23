"""LLDB command script: one entry breakpoint, raw pointers, then continuation.

The offsets are verified from the exact natural-image disassembly, not Rust
layout guesses. The closure's three captures are frame/program/scratch;
scratch+0x48 holds the boxed scalar bank; program+8/+16 is the op slice.
Debugger launches are supplemental and never substitute for stock samples.
"""
import json
import os
from pathlib import Path
import time
import lldb

entered = None


def hit(frame, location, internal):
    global entered
    entered = time.monotonic_ns()
    process = frame.GetThread().GetProcess()
    error = lldb.SBError()
    def word(address):
        value = process.ReadUnsignedFromMemory(address, 8, error)
        if error.Fail():
            raise RuntimeError(str(error))
        return value
    closure = frame.FindRegister('rdi').GetValueAsUnsigned()
    program, scratch = word(closure + 8), word(closure + 16)
    print('T073_ADDRESS ' + json.dumps({'pc': frame.GetPC(),
          'sp': frame.GetSP(), 'fp': frame.GetFP(), 'closure': closure,
          'program': program, 'ops': word(program + 8), 'op_count': word(program + 16),
          'scratch': scratch, 'bank': word(scratch + 0x48)}), flush=True)
    return False


def run(debugger, command, result, internal):
    binary, count = command.split()
    target = debugger.CreateTarget(binary)
    name = '_RNCINvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute3runNtNtNtNtB8_10compact_fn4wide10loop_frame13WideLoopFrameEs_0Ba_'
    breakpoint = target.BreakpointCreateByName(name)
    print('T073_BREAKPOINT', breakpoint.GetNumLocations(), flush=True)
    breakpoint.SetOneShot(True)
    breakpoint.SetScriptCallbackFunction(__name__ + '.hit')
    launch = lldb.SBLaunchInfo(['--raw', str(Path('benchmarks/workloads/broad-micro.js').resolve()), 'local_read', count])
    launch.SetWorkingDirectory(os.getcwd())
    # LLDB normally disables ASLR. Explicitly retain stock ASLR here.
    launch.SetLaunchFlags(launch.GetLaunchFlags() & ~lldb.eLaunchFlagDisableASLR)
    launch.AddOpenFileAction(1, os.environ['T073_STDOUT'], False, True)
    launch.AddOpenFileAction(2, os.environ['T073_STDERR'], False, True)
    error = lldb.SBError()
    debugger.SetAsync(False)
    start = time.monotonic_ns()
    process = target.Launch(launch, error)
    print('T073_EXIT', process.GetState(), process.GetExitStatus(), time.monotonic_ns() - start, 'after_entry_ns', time.monotonic_ns() - entered if entered else None, str(error), flush=True)


def __lldb_init_module(debugger, internal):
    debugger.HandleCommand('command script add -f ' + __name__ + '.run t073')
