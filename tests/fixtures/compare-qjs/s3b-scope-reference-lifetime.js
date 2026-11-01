(function () {
    let x = 1;
    function repeated() {
        let output = '';
        for (let i = 0; i < 3; i++) {
            x ||= (eval('var x=9'), 4);
            output += x;
            if (i === 0) eval('var x=0');
        }
        return output;
    }
    function abrupt() {
        for (let i = 0; i < 2; i++) {
            try { x = i === 0 ? (() => { throw 1; })() : (eval('var x=2'), 3); }
            catch (e) { eval('var x=8'); }
        }
        return x;
    }
    const object = { f() { return this === object; } };
    let receiver;
    with (object) { receiver = f(); }
    return repeated() + ':' + abrupt() + ':' + x + ':' + receiver;
})()
