(function () {
    globalThis.s3bLiveGlobal = 1;
    const read = Function('callback', 'callback(); return s3bLiveGlobal;');
    const first = read(() => { globalThis.s3bLiveGlobal = 9; });
    let calls = 0;
    const second = read(() => {
        Object.defineProperty(globalThis, 's3bLiveGlobal', {
            get() { calls++; return 7; }, configurable: true
        });
    });
    delete globalThis.s3bLiveGlobal;
    let missing;
    try { read(() => {}); } catch (error) { missing = error.name; }
    return first + ':' + second + ':' + calls + ':' + missing;
})()
