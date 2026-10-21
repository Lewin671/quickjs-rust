/* Standalone read-only libproc capability check; no engine linkage or PMU setup. */
#include <sys/resource.h>
#include <sys/wait.h>
#include <libproc.h>
#include <stddef.h>
#include <stdio.h>
#include <stdint.h>
#include <errno.h>
#include <unistd.h>
#include <string.h>
int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "--fixture")) {
        volatile uint64_t n = 0;
        for (uint64_t i = 0; i < 100000; ++i) n += i;
        return n == 4999950000ULL ? 0 : 2;
    }
    struct rusage_info_v4 before = {0}, after = {0};
    int self = proc_pid_rusage(getpid(), RUSAGE_INFO_V4, (rusage_info_t *)&before);
    pid_t child = fork();
    if (child == 0) { execl(argv[0], argv[0], "--fixture", (char *)0); _exit(127); }
    if (child < 0) return 3;
    siginfo_t info = {0};
    int waited = waitid(P_PID, child, &info, WEXITED | WNOWAIT);
    errno = 0;
    int read = proc_pid_rusage(child, RUSAGE_INFO_V4, (rusage_info_t *)&after);
    int err = errno, status = 0;
    waitpid(child, &status, 0);
    printf("{\"self_return\":%d,\"waitid_return\":%d,\"zombie_return\":%d,\"errno\":%d,\"child_exit\":%d,\"self_cycles\":%llu,\"self_instructions\":%llu,\"child_cycles\":%llu,\"child_instructions\":%llu,\"sizeof_v4\":%zu,\"cycles_offset\":%zu,\"instructions_offset\":%zu}\n", self, waited, read, err, WIFEXITED(status) ? WEXITSTATUS(status) : -1, before.ri_cycles, before.ri_instructions, after.ri_cycles, after.ri_instructions, sizeof(before), offsetof(struct rusage_info_v4, ri_cycles), offsetof(struct rusage_info_v4, ri_instructions));
    return 0;
}
