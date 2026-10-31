(function () {
    var count = 0, reads = 0, out = [];
    Object.defineProperty(globalThis, 's3bAccessor', {
        get() { reads++; return count; },
        set(v) { count += v; },
        configurable: true
    });
    function alias(o) { o.s3bAccessor = 1; o.s3bAccessor = 1; return s3bAccessor; }
    function direct() { globalThis.s3bAccessor = 1; return s3bAccessor; }
    function identifier() { s3bAccessor = 1; return s3bAccessor; }
    out.push(alias(globalThis), direct(), identifier(), reads, count);
    delete globalThis.s3bAccessor;
    return out.join(':');
})()
