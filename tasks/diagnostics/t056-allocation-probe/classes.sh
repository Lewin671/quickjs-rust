set -euo pipefail
cat > "$BENCH_OUT/classes.c" <<'C'
#include <malloc/malloc.h>
#include <stdio.h>
#include <stdlib.h>
int main(void) {
  puts("request,good_size,usable_size");
  for (size_t n=1;n<=4096;n++) {
    void *p=malloc(n); if (!p) return 1;
    printf("%zu,%zu,%zu\n",n,malloc_good_size(n),malloc_size(p));
    free(p);
  }
  return 0;
}
C
clang -O2 "$BENCH_OUT/classes.c" -o "$BENCH_OUT/classes"
"$BENCH_OUT/classes" > "$BENCH_OUT/classes.csv"
sw_vers > "$BENCH_OUT/host.txt"
python3 - "$BENCH_OUT/classes.csv" <<'PY'
import csv,sys
r={int(x['request']):x for x in csv.DictReader(open(sys.argv[1]))}
for n in [48,64,72,80,88,112,120,128,136,144,152,160,192,256,264,272,296,304,320,1024,1032]:print(r[n])
PY
