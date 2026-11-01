(() => {
    class Base { static value() { return 3; } }
    let Top = Base;
    const parents = [];
    for (let i = 0; i < 64; i++) {
        parents.push(Top);
        Top = class extends Top {};
        if (Object.getPrototypeOf(Top) !== parents[i]) throw new Error('identity');
    }
    Base.value = function () { return 7; };
    let cycle = false;
    try { Object.setPrototypeOf(Base, Top); } catch (e) { cycle = e instanceof TypeError; }
    class Null extends null {}
    return [Top.value(), cycle, Object.getPrototypeOf(Null) === Function.prototype,
            Object.getPrototypeOf(Null.prototype) === null].join(':');
})()
