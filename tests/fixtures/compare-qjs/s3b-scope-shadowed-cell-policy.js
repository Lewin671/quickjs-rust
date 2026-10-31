(() => {
    const x = 1;
    function outer(x) { return () => ++x; }
    const write = outer(8);
    return [write(), write(), x].join(":");
})()
