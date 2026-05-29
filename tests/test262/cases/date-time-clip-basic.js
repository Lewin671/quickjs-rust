// Derived from: test/built-ins/Date/TimeClip_negative_zero.js
if (!Object.is(new Date(-0).getTime(), 0)) { throw; }
if (!Number.isNaN(new Date(8640000000000001).getTime())) { throw; }
if (!Number.isNaN(new Date(Infinity).getTime())) { throw; }
if (new Date(97445006.9).getTime() !== 97445006) { throw; }
if (!Number.isNaN(Date.UTC(275760, 8, 13, 0, 0, 0, 1))) { throw; }
if (!Number.isNaN(new Date(275760, 8, 13, 0, 0, 0, 1).getTime())) { throw; }
