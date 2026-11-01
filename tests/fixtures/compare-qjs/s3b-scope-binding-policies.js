(function () {
    function immutable() {
        const value = 3;
        try { eval('value = 3'); } catch (error) { return error.name + ':' + value; }
        return 'missing error';
    }
    function mapped(value) {
        eval('arguments[0] = 2');
        var first = value;
        value = 4;
        return first + ':' + eval('arguments[0]');
    }
    function duplicate(value, value) {
        eval('arguments[0] = 8; arguments[1] = 9');
        return value + ':' + arguments[0] + ':' + arguments[1];
    }
    return [immutable(), mapped(1), duplicate(1, 2)].join('|');
})()
