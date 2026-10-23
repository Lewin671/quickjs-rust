/* Diagnostic Darwin SETEXEC launcher. The private ASLR flag is local to the
 * spawned process; no machine setting changes. Keep both modes on this path.
 */
#include <errno.h>
#include <spawn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
extern char **environ;
int main(int argc, char **argv) {
    if (argc < 5) return 2;
    long pad = strtol(argv[2], NULL, 10);
    if (pad >= 0) {
        if (pad > 65536) return 2;
        char *value = malloc((size_t)pad + 1);
        if (!value) return 2;
        memset(value, 'x', (size_t)pad); value[pad] = 0;
        if (setenv("T073_PAD", value, 1)) return 2;
        free(value);
    }
    posix_spawnattr_t attr;
    int error = posix_spawnattr_init(&attr);
    if (error) return error;
    /* XNU bsd/sys/spawn.h: POSIX_SPAWN_DISABLE_ASLR = 0x0100. */
    short flags = POSIX_SPAWN_SETEXEC;
    if (!strcmp(argv[1], "off")) flags |= 0x0100;
    error = posix_spawnattr_setflags(&attr, flags);
    if (!error) error = posix_spawn(NULL, argv[3], NULL, &attr, argv + 3, environ);
    fprintf(stderr, "SETEXEC failed: %s\n", strerror(error));
    return 2;
}
