// Derived from: test/built-ins/Array/prototype/join/S15.4.4.5_A3.1_T1.js
if ([1, "x", true].join() !== "1,x,true") { throw; }
if ([1, 2, 3].join("|") !== "1|2|3") { throw; }
