#!/bin/sh
# Prepared for the user to inspect/run on the iMac; never run by the agent/job.
set -eu
binary="$HOME/Code/co-work-space/tmp/bench-queue/20261021T145905Z-t056-kpc-tool-audit-final/out/qjs-kpc"
rule="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/qjs-kpc.sudoers"
test "$(/usr/bin/id -un)" = qingyingliu
test "$(/usr/bin/stat -f '%u:%Lp' /usr/local)" = '0:755'
/usr/bin/printf 'e3d2bc4c7af7dab8c1181b78ca291429c9ef275e44b20a828e1ca01f1a7ec065  %s\n' "$binary" | /usr/bin/shasum -a 256 -c -
/usr/bin/sudo /usr/sbin/visudo -cf "$rule"
/usr/bin/sudo /usr/bin/install -d -o root -g wheel -m 0755 /usr/local/libexec
/usr/bin/sudo /usr/bin/install -o root -g wheel -m 0755 "$binary" /usr/local/libexec/qjs-kpc
/usr/bin/sudo /usr/bin/install -o root -g wheel -m 0440 "$rule" /private/etc/sudoers.d/qjs-kpc
/usr/bin/sudo /usr/sbin/visudo -cf /private/etc/sudoers
/usr/bin/stat -f '%Su:%Sg %Lp %N' /usr/local/libexec/qjs-kpc /private/etc/sudoers.d/qjs-kpc
