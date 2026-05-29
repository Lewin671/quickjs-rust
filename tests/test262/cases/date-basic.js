// Derived from: test/built-ins/Date/length.js
if (typeof Date !== "function") { throw; }
if (Date.length !== 7) { throw; }
if (Date.parse.length !== 1) { throw; }
if (Date.UTC.length !== 7) { throw; }
if (Date.prototype.getTime.length !== 0) { throw; }
if (new Date(0).getTime() !== 0) { throw; }
if (new Date(0).valueOf() !== 0) { throw; }
if (new Date(0).toISOString() !== "1970-01-01T00:00:00.000Z") { throw; }
