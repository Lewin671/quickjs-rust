Object.hasOwn({ value: 1 }, "value") + ":" + Object.hasOwn(Object.create({ value: 1 }), "value") + ":" + Object.hasOwn("ab", "1") + ":" + Object.hasOwn([1, 2], "length")
