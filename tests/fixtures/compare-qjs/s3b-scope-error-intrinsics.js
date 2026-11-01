(function () {
    const originals = [TypeError, RangeError, ReferenceError, SyntaxError, URIError];
    const suppressed = SuppressedError;
    const source = Object.getOwnPropertyDescriptor(RegExp.prototype, 'source').get;
    let calls = 0;
    TypeError = RangeError = ReferenceError = SyntaxError = URIError = SuppressedError = function () { calls++; };
    const checks = [];
    try { null.value; } catch (error) { checks.push(Object.getPrototypeOf(error) === originals[0].prototype); }
    try { new Array(-1); } catch (error) { checks.push(Object.getPrototypeOf(error) === originals[1].prototype); }
    try { s3bNoBinding; } catch (error) { checks.push(Object.getPrototypeOf(error) === originals[2].prototype); }
    try { eval('let ='); } catch (error) { checks.push(Object.getPrototypeOf(error) === originals[3].prototype); }
    try { decodeURIComponent('%'); } catch (error) { checks.push(Object.getPrototypeOf(error) === originals[4].prototype); }
    try { source.call({}); } catch (error) { checks.push(Object.getPrototypeOf(error) === originals[0].prototype); }
    try {
        using resource = { [Symbol.dispose]() { throw 'dispose'; } };
        throw 'body';
    } catch (error) {
        checks.push(Object.getPrototypeOf(error) === suppressed.prototype && error.error === 'dispose' && error.suppressed === 'body');
    }
    return checks.join(':') + ':' + calls;
})()
