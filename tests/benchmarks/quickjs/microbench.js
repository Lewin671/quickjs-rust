/*
 * QuickJS microbenchmark subset for the current quickjs-rust CLI.
 *
 * Derived from third_party/quickjs-ng/tests/microbench.js at
 * f7830186043e4488f2998759d60a514faf07cbc9.
 *
 * This first-party subset keeps the public benchmark names and output shape,
 * but avoids QuickJS-specific qjs:std/qjs:os modules and unsupported benchmark
 * surfaces. Expand the name table as parser/runtime coverage grows.
 */

function padLeft(value, width) {
    var text = "" + value;
    while (text.length < width) {
        text = " " + text;
    }
    return text;
}

function toFixed2(value) {
    var rounded = Math.round(value * 100);
    var whole = Math.floor(rounded / 100);
    var fraction = "" + (rounded - whole * 100);
    while (fraction.length < 2) {
        fraction = "0" + fraction;
    }
    return whole + "." + fraction;
}

var globalValue = 1;
var globalObject = { a: 1, b: 2, c: 3 };
var globalArray = [1, 2, 3, 4];
var globalResult = 0;

function runNamed(name, n) {
    var i, object, array, sum, x, s;

    if (name === "empty_loop") {
        for (i = 0; i < n; i++) {
        }
        return n;
    }

    if (name === "date_now") {
        for (i = 0; i < n; i++) {
            globalResult = Date.now();
        }
        return n;
    }

    if (name === "prop_read") {
        object = globalObject;
        sum = 0;
        for (i = 0; i < n; i++) {
            sum += object.a;
            sum += object.b;
            sum += object.c;
        }
        globalResult = sum;
        return n * 3;
    }

    if (name === "prop_write") {
        object = {};
        for (i = 0; i < n; i++) {
            object.a = i;
            object.b = i + 1;
            object.c = i + 2;
        }
        globalResult = object.c;
        return n * 3;
    }

    if (name === "array_read") {
        array = globalArray;
        sum = 0;
        for (i = 0; i < n; i++) {
            sum += array[0];
            sum += array[1];
            sum += array[2];
            sum += array[3];
        }
        globalResult = sum;
        return n * 4;
    }

    if (name === "array_write") {
        array = [0, 0, 0, 0];
        for (i = 0; i < n; i++) {
            array[0] = i;
            array[1] = i + 1;
            array[2] = i + 2;
            array[3] = i + 3;
        }
        globalResult = array[3];
        return n * 4;
    }

    if (name === "global_read") {
        sum = 0;
        for (i = 0; i < n; i++) {
            sum += globalValue;
        }
        globalResult = sum;
        return n;
    }

    if (name === "global_write") {
        for (i = 0; i < n; i++) {
            globalValue = i;
        }
        return n;
    }

    if (name === "int_arith") {
        x = 0;
        for (i = 0; i < n; i++) {
            x = (x + i) | 0;
            x = (x * 3) | 0;
            x = (x ^ i) | 0;
        }
        globalResult = x;
        return n * 3;
    }

    if (name === "float_arith") {
        x = 1.5;
        for (i = 0; i < n; i++) {
            x = x + 1.25;
            x = x * 1.125;
            x = x / 1.0625;
        }
        globalResult = x;
        return n * 3;
    }

    if (name === "math_min") {
        x = 0;
        for (i = 0; i < n; i++) {
            x += Math.min(i, 7);
        }
        globalResult = x;
        return n;
    }

    if (name === "string_build") {
        s = "";
        for (i = 0; i < n; i++) {
            s += "x";
            if (s.length > 64) {
                s = "";
            }
        }
        globalResult = s.length;
        return n;
    }

    if (name === "string_slice") {
        s = "the quick brown fox jumps over the lazy dog";
        for (i = 0; i < n; i++) {
            globalResult = s.slice(4, 19).length;
        }
        return n;
    }

    if (name === "int_to_string") {
        for (i = 0; i < n; i++) {
            s = "" + i;
        }
        globalResult = s.length;
        return n;
    }

    if (name === "string_to_int") {
        x = 0;
        for (i = 0; i < n; i++) {
            x += parseInt("12345", 10);
        }
        globalResult = x;
        return n;
    }

    throw "unknown benchmark: " + name;
}

function runOne(name) {
    var n, elapsed, operations, start, end;
    n = 1;
    elapsed = 0;
    operations = 0;
    while (elapsed < 5 && n < 10000) {
        start = Date.now();
        operations = runNamed(name, n);
        end = Date.now();
        elapsed = end - start;
        if (elapsed < 5) {
            n *= 2;
        }
    }
    if (elapsed <= 0) {
        elapsed = 1;
    }
    return [name, operations, elapsed * 1000000 / operations];
}

function runBenchmarks() {
    var names, selected, i, j, name, row, lines;
    names = [
        "empty_loop",
        "date_now",
        "prop_read",
        "prop_write",
        "array_read",
        "array_write",
        "global_read",
        "global_write",
        "int_arith",
        "float_arith",
        "math_min",
        "string_build",
        "string_slice",
        "int_to_string",
        "string_to_int"
    ];
    selected = [];
    for (i = 1; i < scriptArgs.length; i++) {
        name = scriptArgs[i];
        for (j = 0; j < names.length; j++) {
            if (names[j].startsWith(name)) {
                selected.push(names[j]);
            }
        }
    }
    if (selected.length === 0) {
        selected = names;
    }

    lines = [];
    lines.push("                  TEST          N  TIME (ns)");
    for (i = 0; i < selected.length; i++) {
        row = runOne(selected[i]);
        lines.push(padLeft(row[0], 22) + " " + padLeft(row[1], 10) + " " + padLeft(toFixed2(row[2]), 10));
    }
    return lines.join("\n");
}

var microbenchOutput = runBenchmarks();
if (typeof console !== "undefined") {
    console.log(microbenchOutput);
}
microbenchOutput;
