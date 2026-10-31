(() => {
    function make(value) { return [() => arguments, n => value = n]; }
    const pair = make(2, 3);
    pair[1](9);
    const first = pair[0](), second = pair[0]();
    function split(read = () => arguments) {
        var arguments;
        const initial = arguments, same = read() === initial; arguments = 'body';
        return [same, initial.length, arguments].join(':');
    }
    function changed(value = (arguments = 11)) { var arguments; return arguments; }
    function removed() { return delete arguments; }
    function* g(value) { yield () => arguments; value = 8; }
    const it = g(1), read = it.next().value;
    it.next();
    return [first === second, first.length, first[0], first[1], split(undefined, 7), changed(), removed(), read()[0]].join('|');
})()
