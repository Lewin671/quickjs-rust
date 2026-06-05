typeof String.raw + ":" +
String.raw.length + ":" +
Object.getOwnPropertyDescriptor(String, "raw").enumerable + ":" +
String.raw({ raw: ["a", "b", "c"] }, 1, 2) + ":" +
String.raw({ raw: { length: 0 } }) + ":" +
String.raw({ raw: { 0: "x", 1: "y", 2: "z", length: 3 } }, "A")
