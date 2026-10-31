(() => {
function outer(value = eval('arguments[0]')) {
    let arguments = ['body'];
    return [value, (() => () => eval('arguments[0]'))()()];
}
function nested(x) {
    return function (y) { return eval('arguments[0]'); };
}
function own(x) { return () => () => eval('arguments[0]'); }
return [outer('parameter').join(':'), nested(1)(2), own(3)()()].join('|');
})()
