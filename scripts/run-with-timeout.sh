#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <seconds> <command> [args...]" >&2
  exit 2
fi

timeout_seconds="$1"
shift

case "$timeout_seconds" in
  ''|*[!0-9]*)
    echo "error: timeout must be a positive integer: $timeout_seconds" >&2
    exit 2
    ;;
  0)
    echo "error: timeout must be greater than zero" >&2
    exit 2
    ;;
esac

if command -v timeout >/dev/null 2>&1; then
  timeout "$timeout_seconds" "$@"
  exit $?
fi

perl -MPOSIX=':sys_wait_h' -e '
  my $timeout = shift @ARGV;
  my @command = @ARGV;
  my $pid = fork();
  die "fork failed: $!\n" unless defined $pid;

  if ($pid == 0) {
    setpgrp(0, 0);
    exec @command or die "exec failed: $!\n";
  }

  my $timed_out = 0;
  local $SIG{ALRM} = sub {
    $timed_out = 1;
    kill "TERM", -$pid;
    sleep 1;
    kill "KILL", -$pid;
  };

  alarm $timeout;
  waitpid($pid, 0);
  my $status = $?;
  alarm 0;

  exit 124 if $timed_out;
  exit WEXITSTATUS($status) if WIFEXITED($status);
  exit 128 + WTERMSIG($status) if WIFSIGNALED($status);
  exit 1;
' "$timeout_seconds" "$@"
