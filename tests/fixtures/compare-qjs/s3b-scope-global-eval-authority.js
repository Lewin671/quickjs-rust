(() => {
    (0, eval)('var s3bEvalVar; function s3bEvalFunction() { return 7; }');
    const order = Object.getOwnPropertyNames(globalThis).filter(name => name === 's3bEvalVar' || name === 's3bEvalFunction').sort().join(',');
    const shadow = (0, eval)('let s3bEvalFunction = 11; s3bEvalFunction');
    let reads = 0, writes = 0;
    Object.defineProperty(globalThis, 's3bEvalAccessor', {
        get() { reads++; return 3; }, set(v) { writes += v; }, configurable: true
    });
    const first = (0, eval)('var s3bEvalAccessor; s3bEvalAccessor');
    const sum = (0, eval)('var s3bEvalSum = 0; for (var s3bEvalIndex = 0; s3bEvalIndex < 2; s3bEvalIndex++) s3bEvalSum += s3bEvalAccessor; s3bEvalSum');
    (0, eval)('s3bEvalAccessor = 5');
    return [order, shadow, first, sum, reads, writes, s3bEvalFunction()].join(':');
})()
