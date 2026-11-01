(() => {
    eval('var value = 1');
    function inner() {
        eval('var value = (delete value, eval("var value = 2"), 9)');
        return value;
    }
    return inner() + ':' + value;
})()
