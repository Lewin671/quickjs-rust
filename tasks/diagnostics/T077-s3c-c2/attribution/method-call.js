// T077 S3c C2 attribution: method call with `this` on the wide tier (hash-map shape).
// Each segment entry in C2 re-derives strict/receiver via ContextOwner::peek and clones
// the receiver (wide/activation.rs execute_segment); entry binds the receiver into the
// call prefix (call_seed.rs bind/set_receiver). Usage: qjs method-call.js N
function Entry(k, v) { this.key = k; this.value = v; this.next = null; }
Entry.prototype.matches = function (k) {
    var e = this;
    while (e !== null) { if (e.key === k) return e.value; e = e.next; }
    return null;
};
function run(n) {
    var e = new Entry("k", 1), s = 0;
    for (var i = 0; i < n; i++) s += e.matches("k");
    return s;
}
run(Number(scriptArgs[1] || 1000000));
