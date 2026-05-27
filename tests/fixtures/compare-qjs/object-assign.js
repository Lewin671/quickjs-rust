(function () { var target = { a: 1 }; var result = Object.assign(target, "xy", { a: 5, b: 6 }, null); return (result === target) + ":" + result[0] + result[1] + ":" + result.a + ":" + result.b; })()
