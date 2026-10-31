(function () {
    const output = [], Aggregate = AggregateError;
    function C(executor) {
        executor(
            values => output.push(values.map(value => typeof value === 'object' ? value.status + ':' + value.value : value).join(',')),
            error => output.push((error instanceof Aggregate) + ':' + error.errors.join(','))
        );
    }
    C.resolve = value => ({ then(yes) { yes(value); yes(999); } });
    Promise.all.call(C, [1, 2]);
    Promise.allSettled.call(C, [3, 4]);
    AggregateError = function Replacement() { throw 'rebound'; };
    C.resolve = value => ({ then(yes, no) { no(value); no(999); } });
    Promise.any.call(C, [5, 6]);
    return output.join('|');
})()
