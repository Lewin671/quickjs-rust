// T077 S3c C2 attribution: a typed-loop decline becomes permanent for the owning record.
// The outer loop's typed region deoptimizes while `k < 30` (string work), then could run
// natively. S3b re-examines it on each interpreter resume; C2 keeps
// FrameState::declined_typed_loop_programs set across resumes of the same record
// (vm_frame_init.rs:151, general_initialized fast path), so the rest runs on the wide tier.
// Compare `/usr/bin/time -l` instructions: base 8322fc98 vs 63b2347f.
function f(n) {
    var a = Array(n), b = Array(n), k = 0, s = 0, t = "";
    for (var i = 0; i < n; i++) a[i] = i;
    while (true) {
        if (k < 30) { t = ""; for (var i = 0; i < n; i++) t += (a[i] + 1).toString(); }
        k++;
        for (var i = 0; i < n; i++) b[i] = a[i];
        for (var j = 0; j < n; j++) s += b[j] & 7;
        if (k == 200000) return s;
    }
}
f(8);
